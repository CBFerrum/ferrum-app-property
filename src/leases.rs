//! Leases: a unit's days are held by one lease at a time. Activating a lease lets its unit; ending
//! one early by agreement frees it now; notice given ends it once the notice runs out; one past its
//! end is ended by upkeep. Upkeep invoices each lease's periods once each, rent and the other
//! charges, through ERP to the lessee, from its start up to today and its last day: a period cut
//! short (at the end, or ended early) owes its days' share, and each period's rent is the one in
//! effect on its first day, after any review. An issued rent invoice still owed past its due day
//! and the days the settings allow is charged a late fee, once. A deposit is owed and held through
//! the books when the lease starts, and given back, less what was kept and why, once it is over.

use crate::{date, obj, refresh_unit};
use async_trait::async_trait;
use chrono::{Duration, Months, NaiveDate};
use ferrum_domain::util::{d, elevated, id, money, round, rows, s};
use ferrum_kernel::{Ctx, Doc, DocHook, Error, HookCtx, Kernel, Result, Tx};
use ferrum_meta::effective::Schema;
use rust_decimal::Decimal;
use serde_json::{Value, json};
use std::sync::Arc;
use uuid::Uuid;

pub struct LeaseHook;

/// How many months a period of `frequency` lasts.
pub(crate) fn months_of(frequency: &str) -> u32 {
    match frequency {
        "Quarterly" => 3,
        "Yearly" => 12,
        _ => 1,
    }
}

/// The period starting `from`: its last day, cut at the lease's last day.
pub(crate) fn period_end(from: NaiveDate, months: u32, last: NaiveDate) -> Option<NaiveDate> {
    let next = from.checked_add_months(Months::new(months))?;
    Some((next - Duration::days(1)).min(last))
}

/// The period from `from` to `to`: its days, and the days a whole period from `from` has.
pub(crate) fn days_of(from: NaiveDate, to: NaiveDate, months: u32) -> (i64, i64) {
    let whole = period_end(from, months, NaiveDate::MAX).map_or(1, |e| (e - from).num_days() + 1);
    (((to - from).num_days() + 1).clamp(0, whole), whole)
}

/// `amount` for `days` of a whole period's `whole`, to the cent; all of it for a whole period.
pub(crate) fn share(amount: Decimal, days: i64, whole: i64) -> Decimal {
    if days >= whole || whole <= 0 { amount } else { round(amount * Decimal::from(days) / Decimal::from(whole), 2) }
}

/// The rent in effect on `day`: the latest review from that day or before, else the lease's own.
pub(crate) fn rent_on(lease: &Doc, day: NaiveDate) -> Decimal {
    rows(lease, "rent_reviews")
        .iter()
        .filter_map(|r| Some((date(r, "effective_from")?, d(r, "rent"))))
        .filter(|(from, _)| *from <= day)
        .max_by_key(|(from, _)| *from)
        .map_or_else(|| d(lease, "rent"), |(_, rent)| rent)
}

/// The lease's last day: its end, or sooner when it was ended early or notice was given.
pub(crate) fn last_day(lease: &Doc) -> Option<NaiveDate> {
    let end = date(lease, "ends_on")?;
    Some([date(lease, "terminated_on"), date(lease, "leaving_on")].into_iter().flatten().fold(end, NaiveDate::min))
}

/// The settings' company and the accounts a deposit moves between: (company, the lessee's
/// receivable account, the deposit account, the deductions account).
async fn deposit_books(tx: &mut Tx, tenant: Uuid, lessee: Option<Uuid>) -> Result<Option<(Uuid, Option<Uuid>, Uuid, Option<Uuid>)>> {
    let row: Option<(Option<Uuid>, Option<Uuid>, Option<Uuid>, Option<Uuid>)> = sqlx::query_as(
        "select s.company, coalesce(c.receivable_account, co.default_receivable_account), s.deposit_account, s.deposit_deductions_account \
         from e_property_settings s left join e_company co on co.tenant_id = s.tenant_id and co.id = s.company \
         left join e_customer c on c.tenant_id = s.tenant_id and c.id = $2 where s.tenant_id = $1 limit 1",
    )
    .bind(tenant)
    .bind(lessee)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(match row {
        Some((Some(company), receivable, Some(held), deductions)) => Some((company, receivable, held, deductions)),
        _ => None,
    })
}

/// A journal entry line on `account`, the lessee's when `customer` is given.
fn line(account: Uuid, customer: Option<&str>, debit: Decimal, credit: Decimal) -> Value {
    let mut l = json!({"account": account.to_string(), "debit": money(debit), "credit": money(credit)});
    if let Some(c) = customer {
        l["party_type"] = json!("Customer");
        l["customer"] = json!(c);
    }
    l
}

#[async_trait]
impl DocHook for LeaseHook {
    fn name(&self) -> &'static str {
        "property.lease"
    }

    async fn validate(&self, _hc: &mut HookCtx<'_>, doc: &Doc) -> Result<()> {
        if let (Some(f), Some(t)) = (date(doc, "starts_on"), date(doc, "ends_on"))
            && t <= f
        {
            return Err(Error::field("ends_on", "is after the lease starts"));
        }
        if d(doc, "rent") <= Decimal::ZERO {
            return Err(Error::field("rent", "is above zero"));
        }
        let mut seen = std::collections::HashSet::new();
        for r in rows(doc, "rent_reviews") {
            let on = date(&r, "effective_from").ok_or_else(|| Error::field("rent_reviews", "each review says from when"))?;
            if date(doc, "starts_on").is_some_and(|f| on <= f) || date(doc, "ends_on").is_some_and(|t| on > t) {
                return Err(Error::field("rent_reviews", format!("a review from {on} falls after the lease starts and within it")));
            }
            if d(&r, "rent") <= Decimal::ZERO {
                return Err(Error::field("rent_reviews", "a reviewed rent is above zero"));
            }
            if !seen.insert(on) {
                return Err(Error::field("rent_reviews", format!("two reviews from {on}")));
            }
        }
        Ok(())
    }

    async fn validate_change(&self, hc: &mut HookCtx<'_>, old: Option<&Doc>, doc: &Doc) -> Result<()> {
        if hc.ctx.is_system {
            return Ok(());
        }
        if let Some(o) = old
            && s(o, "status") != Some("Draft")
            && ["unit", "lessee", "starts_on", "ends_on", "rent", "frequency", "charges", "rent_reviews", "deposit", "notice_days"]
                .iter()
                .any(|k| o.get(*k) != doc.get(*k))
        {
            return Err(Error::invalid("a lease past its draft keeps its terms: end it and make another"));
        }
        Ok(())
    }

    async fn action(&self, hc: &mut HookCtx<'_>, name: &str, doc: &Doc, params: &Value) -> Result<Option<Value>> {
        let lease = id(doc, "id").ok_or_else(|| Error::internal("a lease without an id"))?;
        let unit = id(doc, "unit").ok_or_else(|| Error::field("unit", "is required"))?;
        let system = elevated(hc.ctx);
        match name {
            "activate" => {
                let from = date(doc, "starts_on").ok_or_else(|| Error::field("starts_on", "is required"))?;
                let to = date(doc, "ends_on").ok_or_else(|| Error::field("ends_on", "is required"))?;
                // Any lease past its draft holds its days, to the day it ended early or ends on notice.
                let other: Option<String> = sqlx::query_scalar(
                    "select number from e_lease where tenant_id = $1 and unit = $2 and status <> 'Draft' and id <> $3 \
                     and starts_on <= $5 and least(ends_on, coalesce(terminated_on, leaving_on, ends_on)) >= $4 limit 1",
                )
                .bind(hc.ctx.tenant)
                .bind(unit)
                .bind(lease)
                .bind(from)
                .bind(to)
                .fetch_optional(&mut **hc.tx)
                .await?;
                if let Some(n) = other {
                    return Err(Error::invalid(format!("the unit is let on some of these days, under {n}")));
                }
                let mut values = obj(json!({"status": "Active", "next_invoice_on": from.to_string()}));
                // The deposit, owed by the lessee and held, through the books when they say where.
                let deposit = d(doc, "deposit");
                if deposit > Decimal::ZERO
                    && let Some((company, receivable, held, _)) = deposit_books(hc.tx, hc.ctx.tenant, id(doc, "lessee")).await?
                {
                    let receivable = receivable.ok_or_else(|| Error::invalid("the company has no receivable account for the deposit"))?;
                    let lessee = s(doc, "lessee");
                    let je = hc
                        .kernel
                        .insert_tx(
                            &system,
                            hc.tx,
                            "JournalEntry",
                            obj(json!({"company": company.to_string(), "remark": format!("Deposit held for lease {}", s(doc, "number").unwrap_or_default()),
                                       "accounts": [line(receivable, lessee, deposit, Decimal::ZERO), line(held, None, Decimal::ZERO, deposit)]})),
                        )
                        .await?;
                    values.insert("deposit_entry".into(), je.get("id").cloned().unwrap_or(Value::Null));
                }
                hc.kernel.set_values_tx(&system, hc.tx, "Lease", lease, values).await?;
                refresh_unit(hc.kernel, &system, hc.tx, unit).await?;
                Ok(Some(json!({"status": "Active"})))
            }
            "give_notice" => {
                let on = params
                    .get("date")
                    .and_then(Value::as_str)
                    .and_then(|v| NaiveDate::parse_from_str(v, "%Y-%m-%d").ok())
                    .ok_or_else(|| Error::field("date", "is the day notice was given, YYYY-MM-DD"))?;
                let by = params
                    .get("by")
                    .and_then(Value::as_str)
                    .filter(|b| ["Lessee", "Landlord"].contains(b))
                    .ok_or_else(|| Error::field("by", "is the lessee or the landlord"))?;
                if date(doc, "starts_on").is_some_and(|f| on < f) || on > hc.ctx.today() {
                    return Err(Error::field("date", "is from the lease's start to today"));
                }
                if let Some(given) = s(doc, "notice_given_on") {
                    return Err(Error::invalid(format!("notice was given on {given}")));
                }
                let leaving = on + Duration::days(doc.get("notice_days").and_then(Value::as_i64).unwrap_or(0));
                let end = date(doc, "ends_on").ok_or_else(|| Error::field("ends_on", "is required"))?;
                if leaving >= end {
                    return Err(Error::invalid(format!("the lease ends on {end}, before the notice runs out: it runs to its end")));
                }
                let values = json!({"notice_given_on": on.to_string(), "notice_by": by, "leaving_on": leaving.to_string()});
                hc.kernel.set_values_tx(&system, hc.tx, "Lease", lease, obj(values)).await?;
                Ok(Some(json!({"leaving_on": leaving.to_string()})))
            }
            "review_rent" => {
                let on = params
                    .get("date")
                    .and_then(Value::as_str)
                    .and_then(|v| NaiveDate::parse_from_str(v, "%Y-%m-%d").ok())
                    .ok_or_else(|| Error::field("date", "is the day the new rent starts, YYYY-MM-DD"))?;
                let rent: Decimal = params
                    .get("rent")
                    .and_then(|v| v.as_str().map(str::to_string).or_else(|| v.as_f64().map(|f| f.to_string())))
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_default();
                if rent <= Decimal::ZERO {
                    return Err(Error::field("rent", "is above zero"));
                }
                // Periods already invoiced keep their rent.
                if date(doc, "next_invoice_on").is_some_and(|next| on < next) || last_day(doc).is_some_and(|l| on > l) {
                    return Err(Error::field("date", "falls on or after the next period to invoice, within the lease"));
                }
                // The reviews made, as rows of their own fields (their ids kept, nothing the table adds).
                let mut reviews: Vec<Doc> = rows(doc, "rent_reviews")
                    .into_iter()
                    .map(|r| r.into_iter().filter(|(k, _)| matches!(k.as_str(), "id" | "effective_from" | "rent" | "note")).collect())
                    .collect();
                if reviews.iter().any(|r| date(r, "effective_from").is_some_and(|f| f >= on)) {
                    return Err(Error::field("date", "is after the reviews already made"));
                }
                reviews.push(obj(json!({"effective_from": on.to_string(), "rent": money(rent), "note": params.get("note").and_then(Value::as_str)})));
                hc.kernel.set_child_rows_tx(&system, hc.tx, "Lease", "rent_reviews", lease, reviews).await?;
                Ok(Some(json!({"rent": money(rent), "from": on.to_string()})))
            }
            "return_deposit" => {
                if s(doc, "deposit_return_entry").is_some() {
                    return Err(Error::invalid("the deposit was given back already"));
                }
                let held_entry = s(doc, "deposit_entry").ok_or_else(|| Error::invalid("no deposit was held through the books for this lease"))?;
                let deposit = d(doc, "deposit");
                let kept: Decimal = params
                    .get("kept")
                    .and_then(|v| v.as_str().map(str::to_string).or_else(|| v.as_f64().map(|f| f.to_string())))
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_default();
                if kept < Decimal::ZERO || kept > deposit {
                    return Err(Error::field("kept", format!("is from nothing to the deposit, {deposit}")));
                }
                let why = params.get("why").and_then(Value::as_str).map(str::trim).filter(|w| !w.is_empty());
                if kept > Decimal::ZERO && why.is_none() {
                    return Err(Error::field("why", "keeping part of a deposit says why"));
                }
                let (company, receivable, held, deductions) =
                    deposit_books(hc.tx, hc.ctx.tenant, id(doc, "lessee")).await?.ok_or_else(|| Error::invalid("the settings name no deposit account"))?;
                let receivable = receivable.ok_or_else(|| Error::invalid("the company has no receivable account for the deposit"))?;
                let mut lines = vec![line(held, None, deposit, Decimal::ZERO)];
                if deposit > kept {
                    lines.push(line(receivable, s(doc, "lessee"), Decimal::ZERO, deposit - kept));
                }
                if kept > Decimal::ZERO {
                    let deductions = deductions.ok_or_else(|| Error::field("kept", "the settings name no account for what is kept of deposits"))?;
                    lines.push(line(deductions, None, Decimal::ZERO, kept));
                }
                let remark = match why {
                    Some(w) => format!("Deposit of lease {} given back, {kept} kept: {w} (held in {held_entry})", s(doc, "number").unwrap_or_default()),
                    None => format!("Deposit of lease {} given back (held in {held_entry})", s(doc, "number").unwrap_or_default()),
                };
                let je = hc
                    .kernel
                    .insert_tx(&system, hc.tx, "JournalEntry", obj(json!({"company": company.to_string(), "remark": remark, "accounts": lines})))
                    .await?;
                let values = json!({"deposit_return_entry": je.get("id"), "deposit_kept": money(kept)});
                hc.kernel.set_values_tx(&system, hc.tx, "Lease", lease, obj(values)).await?;
                Ok(Some(json!({"returned": money(deposit - kept), "kept": money(kept)})))
            }
            "terminate" => {
                let on = params
                    .get("date")
                    .and_then(Value::as_str)
                    .and_then(|v| NaiveDate::parse_from_str(v, "%Y-%m-%d").ok())
                    .ok_or_else(|| Error::field("date", "is the lease's last day, YYYY-MM-DD"))?;
                if date(doc, "starts_on").is_some_and(|f| on < f) || on > hc.ctx.today() {
                    return Err(Error::field("date", "is from the lease's start to today"));
                }
                hc.kernel.set_values_tx(&system, hc.tx, "Lease", lease, obj(json!({"status": "Terminated", "terminated_on": on.to_string()}))).await?;
                refresh_unit(hc.kernel, &system, hc.tx, unit).await?;
                Ok(Some(json!({"status": "Terminated"})))
            }
            _ => Ok(None),
        }
    }
}

/// One workspace's upkeep as of `today`: each lease's due periods invoiced once each, up to its last
/// day, and active leases past their end ended.
pub(crate) async fn upkeep(k: &Kernel, tenant: Uuid, schema: Arc<Schema>, today: NaiveDate) -> Result<()> {
    let ctx = Ctx::system(tenant, schema);
    let mut tx = k.db.tenant_tx(tenant).await?;
    let settings: Option<(Option<Uuid>, Option<Uuid>)> =
        sqlx::query_as("select company, rent_item from e_property_settings where tenant_id = $1 limit 1").bind(tenant).fetch_optional(&mut *tx).await?;
    let Some((Some(company), Some(rent_item))) = settings else {
        tx.commit().await?;
        return Ok(());
    };
    let currency: Option<Uuid> = sqlx::query_scalar("select default_currency from e_company where tenant_id = $1 and id = $2")
        .bind(tenant)
        .bind(company)
        .fetch_optional(&mut *tx)
        .await?
        .flatten();
    let leases: Vec<Uuid> = sqlx::query_scalar(
        "select id from e_lease where tenant_id = $1 and status in ('Active', 'Terminated') \
         and ((next_invoice_on <= $2 and next_invoice_on <= least(ends_on, coalesce(terminated_on, leaving_on, ends_on))) \
              or (status = 'Active' and least(ends_on, coalesce(leaving_on, ends_on)) < $2)) order by id",
    )
    .bind(tenant)
    .bind(today)
    .fetch_all(&mut *tx)
    .await?;
    for lease in leases {
        let doc = k.get_tx(&ctx, &mut tx, "Lease", lease, true).await?;
        let (Some(mut from), Some(end), Some(last)) = (date(&doc, "next_invoice_on"), date(&doc, "ends_on"), last_day(&doc)) else { continue };
        let months = months_of(s(&doc, "frequency").unwrap_or("Monthly"));
        let charges = rows(&doc, "charges");
        while from <= today && from <= last {
            let Some(to) = period_end(from, months, last) else { break };
            // A period is invoiced once: its row says so.
            let done: bool = sqlx::query_scalar("select exists(select 1 from e_rent_invoice where tenant_id = $1 and lease = $2 and period_from = $3)")
                .bind(tenant)
                .bind(lease)
                .bind(from)
                .fetch_one(&mut *tx)
                .await?;
            if !done {
                // A period cut short owes its days' share; the rent is the one in effect on its first day.
                let (days, whole) = days_of(from, to, months);
                let cut = if days < whole { format!(" ({days} of {whole} days)") } else { String::new() };
                let line = |item: Option<&str>, rate: Decimal, what: &str| json!({"item": item, "qty": "1", "price_list_rate": money(rate), "discount_percent": "0", "description": format!("{what}, {from} to {to}{cut}")});
                let rent_item = rent_item.to_string();
                let rent = share(rent_on(&doc, from), days, whole);
                let mut items = vec![line(Some(&rent_item), rent, "Rent")];
                items.extend(charges.iter().map(|c| line(s(c, "item"), share(d(c, "amount"), days, whole), s(c, "charge").unwrap_or("Charge"))));
                let amount = rent + charges.iter().map(|c| share(d(c, "amount"), days, whole)).sum::<Decimal>();
                let remarks = format!("Lease {}, {from} to {to}", s(&doc, "number").unwrap_or_default());
                let inv = k
                    .insert_tx(
                        &ctx,
                        &mut tx,
                        "SalesInvoice",
                        obj(json!({"company": company.to_string(), "customer": s(&doc, "lessee"), "currency": currency.map(|c| c.to_string()), "items": items, "remarks": remarks})),
                    )
                    .await?;
                k.insert_tx(
                    &ctx,
                    &mut tx,
                    "RentInvoice",
                    obj(json!({"lease": lease.to_string(), "period_from": from.to_string(), "period_to": to.to_string(), "amount": money(amount), "sales_invoice": inv.get("id")})),
                )
                .await?;
            }
            from = to + Duration::days(1);
        }
        let mut values = obj(json!({"next_invoice_on": from.to_string()}));
        let active = s(&doc, "status") == Some("Active");
        // Past the day notice runs out, it is over, early; past its end, it ended.
        let noticed = active && date(&doc, "leaving_on").is_some_and(|l| l < end && l < today);
        let ended = active && (noticed || end < today);
        if noticed {
            values.insert("status".into(), json!("Terminated"));
            values.insert("terminated_on".into(), json!(s(&doc, "leaving_on")));
        } else if ended {
            values.insert("status".into(), json!("Ended"));
        }
        k.set_values_tx(&ctx, &mut tx, "Lease", lease, values).await?;
        if ended && let Some(unit) = id(&doc, "unit") {
            refresh_unit(k, &ctx, &mut tx, unit).await?;
        }
    }
    late_fees(k, &ctx, &mut tx, company, currency, today).await?;
    tx.commit().await?;
    Ok(())
}

/// Each issued rent invoice still owed past its due day and the days the settings allow, charged a
/// late fee once: the fixed fee and a share of what is still owed. How many.
async fn late_fees(k: &Kernel, ctx: &Ctx, tx: &mut Tx, company: Uuid, currency: Option<Uuid>, today: NaiveDate) -> Result<usize> {
    let terms: Option<(Option<Uuid>, Option<i64>, Option<Decimal>, Option<Decimal>)> =
        sqlx::query_as("select late_fee_item, late_fee_after_days, late_fee_amount, late_fee_percent from e_property_settings where tenant_id = $1 limit 1")
            .bind(ctx.tenant)
            .fetch_optional(&mut **tx)
            .await?;
    let Some((Some(item), after, fixed, percent)) = terms else { return Ok(0) };
    let owed: Vec<(Uuid, Uuid, Option<String>, Decimal, NaiveDate)> = sqlx::query_as(
        "select r.id, si.customer, si.number, si.outstanding_amount, coalesce(si.due_date, si.posting_date) from e_rent_invoice r \
         join e_sales_invoice si on si.tenant_id = r.tenant_id and si.id = r.sales_invoice \
         where r.tenant_id = $1 and r.late_fee_invoice is null and si.docstatus = 1 and si.outstanding_amount > 0 \
         and coalesce(si.due_date, si.posting_date) + $2::int < $3 order by r.id",
    )
    .bind(ctx.tenant)
    .bind(i32::try_from(after.unwrap_or(7).max(0)).unwrap_or(i32::MAX))
    .bind(today)
    .fetch_all(&mut **tx)
    .await?;
    let mut n = 0;
    for (rent_invoice, customer, number, outstanding, due) in owed {
        let fee = round(fixed.unwrap_or_default() + outstanding * percent.unwrap_or_default() / Decimal::ONE_HUNDRED, 2);
        if fee <= Decimal::ZERO {
            continue;
        }
        let what = format!("Late fee on {}, due {due}", number.as_deref().unwrap_or("rent"));
        let inv = k
            .insert_tx(
                ctx,
                tx,
                "SalesInvoice",
                obj(json!({"company": company.to_string(), "customer": customer.to_string(), "currency": currency.map(|c| c.to_string()), "remarks": what,
                           "items": [{"item": item.to_string(), "qty": "1", "price_list_rate": money(fee), "discount_percent": "0", "description": what}]})),
            )
            .await?;
        k.set_values_tx(ctx, tx, "RentInvoice", rent_invoice, obj(json!({"late_fee_invoice": inv.get("id")}))).await?;
        n += 1;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::{days_of, last_day, months_of, period_end, rent_on, share};
    use chrono::NaiveDate;
    use rust_decimal::Decimal;
    use serde_json::json;

    #[test]
    fn a_period_cut_short_owes_its_days() {
        let day = |s: &str| NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap();
        let n = |s: &str| s.parse::<Decimal>().unwrap();
        assert_eq!(days_of(day("2026-04-01"), day("2026-05-15"), 3), (45, 91));
        assert_eq!(days_of(day("2026-02-01"), day("2026-02-28"), 1), (28, 28), "a whole February");
        assert_eq!(share(n("90000"), 45, 91), n("44505.49"));
        assert_eq!(share(n("30000"), 31, 31), n("30000"), "a whole period, all of it");
        assert_eq!(share(n("30000"), 0, 31), n("0.00"));
    }

    #[test]
    fn the_rent_in_effect_and_the_last_day() {
        let day = |s: &str| NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap();
        let lease = json!({"rent": "30000", "ends_on": "2026-12-31", "rent_reviews": [{"effective_from": "2026-07-01", "rent": "33000"}, {"effective_from": "2026-04-01", "rent": "31500"}]});
        let lease = lease.as_object().unwrap();
        let rent = |s: &str| rent_on(lease, day(s)).to_string();
        assert_eq!(
            (rent("2026-03-01"), rent("2026-04-01"), rent("2026-06-30"), rent("2026-10-01")),
            ("30000".into(), "31500".into(), "31500".into(), "33000".into())
        );
        assert_eq!(last_day(lease), Some(day("2026-12-31")));
        let noticed = json!({"ends_on": "2026-12-31", "leaving_on": "2026-10-15"});
        assert_eq!(last_day(noticed.as_object().unwrap()), Some(day("2026-10-15")));
        let both = json!({"ends_on": "2026-12-31", "leaving_on": "2026-10-15", "terminated_on": "2026-09-30"});
        assert_eq!(last_day(both.as_object().unwrap()), Some(day("2026-09-30")), "the soonest");
    }

    #[test]
    fn periods_end_the_day_before_the_next_or_with_the_lease() {
        let day = |s: &str| NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap();
        assert_eq!(period_end(day("2026-01-01"), months_of("Monthly"), day("2026-12-31")), Some(day("2026-01-31")));
        assert_eq!(period_end(day("2026-01-31"), months_of("Monthly"), day("2026-12-31")), Some(day("2026-02-27")), "a month on from the 31st is the 28th");
        assert_eq!(period_end(day("2026-01-01"), months_of("Quarterly"), day("2026-12-31")), Some(day("2026-03-31")));
        assert_eq!(period_end(day("2026-11-01"), months_of("Quarterly"), day("2026-12-15")), Some(day("2026-12-15")), "cut at the lease's last day");
        assert_eq!(months_of("Yearly"), 12);
    }
}
