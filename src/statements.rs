//! Owners' statements. A building's owner is owed, for some days, what was collected on its rent
//! invoices for periods starting then (rent, charges and late fees paid, credit notes taken off),
//! less the management fee the company keeps (the building's share) and the expenses charged to
//! them: the repairs of its units done then, at their cost and once each, and any added by hand.
//! While a draft, a statement is worked out again whenever it is saved. Billing the payout makes
//! the owner's purchase invoice to the company, from the owner as ERP's supplier and as the
//! settings' owner payout item, paid as ERP pays bills; the statement is then kept as billed, and
//! its owner sees it on the portal.

use crate::{date, obj};
use async_trait::async_trait;
use chrono::NaiveDate;
use ferrum_domain::flow::created;
use ferrum_domain::util::{d, elevated, id, money, round, rows, s};
use ferrum_kernel::{Doc, DocHook, Error, HookCtx, Result, Tx};
use rust_decimal::Decimal;
use serde_json::{Value, json};
use uuid::Uuid;

pub struct StatementHook;

/// What was collected on `building`'s rent invoices for periods starting from `from` to `to`:
/// invoiced (late fees in, credit notes out) less what is still owed.
async fn collected(tx: &mut Tx, tenant: Uuid, building: Uuid, (from, to): (NaiveDate, NaiveDate)) -> Result<Decimal> {
    Ok(sqlx::query_scalar(
        "select coalesce(sum(si.grand_total), 0) - coalesce(sum(si.outstanding_amount), 0) \
         from e_rent_invoice r join e_lease l on l.tenant_id = r.tenant_id and l.id = r.lease \
         join e_unit u on u.tenant_id = l.tenant_id and u.id = l.unit \
         join e_sales_invoice si on si.tenant_id = r.tenant_id and si.docstatus = 1 and si.id in (r.sales_invoice, r.late_fee_invoice, r.credit_note) \
         where r.tenant_id = $1 and u.building = $2 and r.period_from between $3 and $4",
    )
    .bind(tenant)
    .bind(building)
    .bind(from)
    .bind(to)
    .fetch_one(&mut **tx)
    .await?)
}

/// The repairs of `building`'s units done from `from` to `to` with a cost, not charged on another
/// statement: (repair, its line's words, cost).
async fn repairs(tx: &mut Tx, tenant: Uuid, building: Uuid, (from, to): (NaiveDate, NaiveDate), me: Option<Uuid>) -> Result<Vec<(Uuid, String, Decimal)>> {
    Ok(sqlx::query_as(
        "select r.id, concat('Repair ', r.number, ', unit ', u.unit_number, ': ', r.summary), r.cost \
         from e_repair_request r join e_unit u on u.tenant_id = r.tenant_id and u.id = r.unit \
         where r.tenant_id = $1 and u.building = $2 and r.status = 'Done' and r.done_on between $3 and $4 and coalesce(r.cost, 0) > 0 \
         and not exists (select 1 from e_owner_statement__expenses e where e.tenant_id = r.tenant_id and e.repair = r.id and e.parent_id is distinct from $5) \
         order by r.done_on, r.number",
    )
    .bind(tenant)
    .bind(building)
    .bind(from)
    .bind(to)
    .bind(me)
    .fetch_all(&mut **tx)
    .await?)
}

/// The fee on `collected` at `percent`, and what is left once it and `expenses` are taken off.
pub(crate) fn payout(collected: Decimal, percent: Decimal, expenses: Decimal) -> (Decimal, Decimal) {
    let fee = round(collected * percent / Decimal::ONE_HUNDRED, 2);
    (fee, collected - fee - expenses)
}

#[async_trait]
impl DocHook for StatementHook {
    fn name(&self) -> &'static str {
        "property.statement"
    }

    fn fills(&self) -> &'static [&'static str] {
        &["collected", "fee_percent", "management_fee", "expenses", "expenses_total", "payout"]
    }

    async fn before_validate(&self, hc: &mut HookCtx<'_>, doc: &mut Doc) -> Result<()> {
        if s(doc, "status").is_some_and(|st| st != "Draft") {
            return Ok(());
        }
        let (Some(building), Some(from), Some(to)) = (id(doc, "building"), date(doc, "period_from"), date(doc, "period_to")) else { return Ok(()) };
        let tenant = hc.ctx.tenant;
        let got = collected(hc.tx, tenant, building, (from, to)).await?;
        let percent: Option<Decimal> = sqlx::query_scalar("select management_fee_percent from e_building where tenant_id = $1 and id = $2")
            .bind(tenant)
            .bind(building)
            .fetch_optional(&mut **hc.tx)
            .await?
            .flatten();
        let percent = percent.unwrap_or_default();
        // Lines added by hand stay; the repairs' are worked out again.
        let mut lines: Vec<Value> = rows(doc, "expenses")
            .into_iter()
            .filter(|l| id(l, "repair").is_none())
            .map(|mut l| {
                // A row added by hand keeps its id, or is given one.
                l.entry("id").or_insert_with(|| json!(Uuid::now_v7().to_string()));
                Value::Object(l)
            })
            .collect();
        for (repair, words, cost) in repairs(hc.tx, tenant, building, (from, to), id(doc, "id")).await? {
            lines.push(json!({"id": Uuid::now_v7().to_string(), "description": words, "amount": money(cost), "repair": repair.to_string()}));
        }
        let spent: Decimal = lines.iter().filter_map(Value::as_object).map(|l| d(l, "amount")).sum();
        let (fee, left) = payout(got, percent, spent);
        doc.insert("expenses".into(), Value::Array(lines));
        for (k, v) in [
            ("collected", money(got)),
            ("fee_percent", money(percent)),
            ("management_fee", money(fee)),
            ("expenses_total", money(spent)),
            ("payout", money(left)),
        ] {
            doc.insert(k.into(), v);
        }
        Ok(())
    }

    async fn validate(&self, hc: &mut HookCtx<'_>, doc: &Doc) -> Result<()> {
        if s(doc, "status").is_some_and(|st| st != "Draft") {
            return Ok(());
        }
        let (from, to) = (date(doc, "period_from"), date(doc, "period_to"));
        if let (Some(f), Some(t)) = (from, to)
            && t < f
        {
            return Err(Error::field("period_to", "is on or after the first day"));
        }
        let other: Option<String> = sqlx::query_scalar(
            "select number from e_owner_statement where tenant_id = $1 and building = $2 and id is distinct from $3 \
             and period_from <= $5 and period_to >= $4 limit 1",
        )
        .bind(hc.ctx.tenant)
        .bind(id(doc, "building"))
        .bind(id(doc, "id"))
        .bind(from)
        .bind(to)
        .fetch_optional(&mut **hc.tx)
        .await?
        .flatten();
        if let Some(n) = other {
            return Err(Error::field("period_from", format!("{n} covers some of these days already")));
        }
        Ok(())
    }

    async fn validate_change(&self, hc: &mut HookCtx<'_>, old: Option<&Doc>, _doc: &Doc) -> Result<()> {
        if !hc.ctx.is_system && old.is_some_and(|o| s(o, "status") == Some("Billed")) {
            return Err(Error::invalid("a billed statement is kept as it was billed"));
        }
        Ok(())
    }

    async fn action(&self, hc: &mut HookCtx<'_>, name: &str, doc: &Doc, _params: &Value) -> Result<Option<Value>> {
        if name != "bill_owner" {
            return Ok(None);
        }
        let me = id(doc, "id").ok_or_else(|| Error::internal("a statement without an id"))?;
        let left = d(doc, "payout");
        if left <= Decimal::ZERO {
            return Err(Error::invalid(format!("nothing is owed to the owner: the fee and expenses come to {} more than was collected", money(-left))));
        }
        let owner: Option<Uuid> = sqlx::query_scalar("select owner_supplier from e_building where tenant_id = $1 and id = $2")
            .bind(hc.ctx.tenant)
            .bind(id(doc, "building"))
            .fetch_optional(&mut **hc.tx)
            .await?
            .flatten();
        let owner = owner.ok_or_else(|| Error::invalid("the building names no owner in the books (Owner In The Books) to bill the payout"))?;
        let set: Option<(Option<Uuid>, Option<Uuid>)> =
            sqlx::query_as("select company, owner_payout_item from e_property_settings where tenant_id = $1 limit 1")
                .bind(hc.ctx.tenant)
                .fetch_optional(&mut **hc.tx)
                .await?;
        let Some((Some(company), Some(item))) = set else {
            return Err(Error::invalid("Property Settings name no Owner Payout Item to bill the payout as"));
        };
        let number = s(doc, "number").unwrap_or_default().to_string();
        let what = format!(
            "Owner statement {number}, {} to {}: collected {}, management fee {} ({}%), expenses {}",
            s(doc, "period_from").unwrap_or_default(),
            s(doc, "period_to").unwrap_or_default(),
            money(d(doc, "collected")),
            money(d(doc, "management_fee")),
            d(doc, "fee_percent").normalize(),
            money(d(doc, "expenses_total"))
        );
        let system = elevated(hc.ctx);
        let bill = hc
            .kernel
            .insert_tx(
                &system,
                hc.tx,
                "PurchaseInvoice",
                obj(json!({"company": company.to_string(), "supplier": owner.to_string(), "bill_no": number, "remarks": what,
                           "items": [{"item": item.to_string(), "qty": "1", "price_list_rate": money(left), "discount_percent": "0"}]})),
            )
            .await?;
        let bill_id = id(&bill, "id").ok_or_else(|| Error::internal("a purchase invoice without an id"))?;
        hc.kernel.set_values_tx(&system, hc.tx, "OwnerStatement", me, obj(json!({"status": "Billed", "purchase_invoice": bill_id.to_string()}))).await?;
        Ok(Some(json!({"status": "Billed", "purchase_invoice": created("PurchaseInvoice", &bill)})))
    }
}

#[cfg(test)]
mod tests {
    use super::payout;
    use rust_decimal::Decimal;

    #[test]
    fn the_owners_share() {
        let v = |s: &str| s.parse::<Decimal>().unwrap();
        assert_eq!(payout(v("12000"), v("10"), v("1300")), (v("1200.00"), v("9500.00")));
        assert_eq!(payout(v("999.99"), v("7.5"), v("0")), (v("75.00"), v("924.99")), "the fee to the cent");
        assert_eq!(payout(v("100"), v("0"), v("250")).1, v("-150"), "more spent than collected");
    }
}
