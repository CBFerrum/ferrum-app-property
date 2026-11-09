//! Owner Reconciliation (FEO-050): each building's billed owners' statements beside the books. What
//! the statements charged the owner from bills under the building's cost center against what the
//! books carry there (expense lines under that cost center in the statements' days), and the
//! payouts against the owner's bills made for them and what was paid on those. A building
//! reconciles when both agree. For Property Managers and administrators.

use async_trait::async_trait;
use ferrum_domain::reports::{ReportSource, col, read_tx};
use ferrum_domain::util::money;
use ferrum_kernel::{Ctx, Error, Kernel, Result};
use rust_decimal::Decimal;
use serde_json::{Value, json};
use std::collections::HashMap;
use uuid::Uuid;

pub struct OwnerReconciliation;

#[async_trait]
impl ReportSource for OwnerReconciliation {
    fn catalog(&self) -> Vec<Value> {
        vec![json!({"name": "owner_reconciliation", "label": "Owner Reconciliation", "module": "property", "entity": "OwnerStatement", "params": []})]
    }

    async fn run(&self, k: &Kernel, ctx: &Ctx, name: &str, _params: &HashMap<String, String>) -> Result<Value> {
        if name != "owner_reconciliation" {
            return Err(Error::NotFound(format!("report {name}")));
        }
        if !ctx.is_admin && !ctx.roles.iter().any(|r| r == "Property Manager") {
            return Err(Error::Forbidden("owners' reconciliation is for property managers".into()));
        }
        let mut tx = read_tx(k, ctx).await?;
        #[allow(clippy::type_complexity)]
        let rows: Vec<(Uuid, String, Option<Uuid>, i64, Decimal, Decimal, Decimal, Decimal, Decimal, Decimal, Decimal)> = sqlx::query_as(
            "select b.id, b.building_name, b.cost_center, count(*), coalesce(sum(o.collected), 0), coalesce(sum(o.management_fee), 0), \
                    coalesce(sum(o.expenses_total), 0), coalesce(sum(o.payout), 0), \
                    coalesce(sum(pi.grand_total), 0), coalesce(sum(case when pi.docstatus = 1 then pi.grand_total - pi.outstanding_amount else 0 end), 0), \
                    coalesce(sum(case when pi.docstatus = 1 then pi.outstanding_amount else 0 end), 0) \
             from e_owner_statement o join e_building b on b.tenant_id = o.tenant_id and b.id = o.building \
             left join e_purchase_invoice pi on pi.tenant_id = o.tenant_id and pi.id = o.purchase_invoice \
             where o.tenant_id = $1 and o.status = 'Billed' group by b.id, b.building_name, b.cost_center order by b.building_name limit 1000",
        )
        .bind(ctx.tenant)
        .fetch_all(&mut *tx)
        .await?;
        let mut out = Vec::new();
        for (building, name, cc, statements, collected, fees, expenses, payouts, billed, paid, owed) in rows {
            // Charged from bills under the building's cost center, and what the books carry there
            // in the billed statements' days.
            let (charged, books): (Decimal, Decimal) = match cc {
                Some(cc) => sqlx::query_as(
                    "select \
                       (select coalesce(sum(e.amount), 0) from e_owner_statement__expenses e join e_owner_statement o on o.tenant_id = e.tenant_id and o.id = e.parent_id \
                        where e.tenant_id = $1 and o.building = $2 and o.status = 'Billed' and e.purchase_invoice is not null \
                          and exists (select 1 from e_purchase_invoice__items it where it.tenant_id = e.tenant_id and it.parent_id = e.purchase_invoice and it.cost_center = $3)), \
                       (select coalesce(sum(l.debit - l.credit), 0) from journal_line l join e_account a on a.tenant_id = l.tenant_id and a.id = l.account_id \
                        where l.tenant_id = $1 and l.cost_center_id = $3 and a.root_type = 'Expense' \
                          and exists (select 1 from e_owner_statement o where o.tenant_id = l.tenant_id and o.building = $2 and o.status = 'Billed' \
                                      and l.posting_date between o.period_from and o.period_to))",
                )
                .bind(ctx.tenant)
                .bind(building)
                .bind(cc)
                .fetch_one(&mut *tx)
                .await?,
                None => (Decimal::ZERO, Decimal::ZERO),
            };
            let reconciles = charged == books && payouts == billed;
            out.push(json!({
                "building": name, "statements": statements, "collected": money(collected), "management_fee": money(fees), "expenses": money(expenses),
                "from_bills": money(charged), "books_expenses": cc.map(|_| money(books)), "payouts": money(payouts), "billed_to_company": money(billed),
                "paid": money(paid), "still_to_pay": money(owed), "reconciles": reconciles,
            }));
        }
        tx.commit().await?;
        Ok(json!({
            "columns": [col("building", "Building", "data"), col("statements", "Statements", "int"), col("collected", "Collected", "currency"),
                col("management_fee", "Management Fee", "currency"), col("expenses", "Expenses", "currency"), col("from_bills", "Expenses From Bills", "currency"),
                col("books_expenses", "Books: Expenses", "currency"), col("payouts", "Payouts", "currency"), col("billed_to_company", "Billed By Owner", "currency"),
                col("paid", "Paid", "currency"), col("still_to_pay", "Still To Pay", "currency"), col("reconciles", "Reconciles", "check")],
            "rows": out,
        }))
    }
}
