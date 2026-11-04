//! A year at a block of flats (FERRUM-INDUSTRY-008) against a real PostgreSQL
//! (`FERRUM_PROPERTY_TEST_DATABASE_URL`, default `postgres://ferrum:ferrum@localhost:5432/ferrum_property_test`;
//! reset first): a lease activated, letting its unit, and its terms kept; a second lease on the same
//! days refused; upkeep invoicing each month's rent and parking once through ERP, then ending the
//! lease and freeing the unit; a quarterly lease's last period cut at its end; a lease ended early,
//! invoiced to its last day; urgent requests taking a unit out of service until done; who may do
//! what; and the portal's "Your home", where a lessee sees only their leases and asks for repairs
//! only in a unit they rent now.

use ferrum_kernel::secret::SecretKey;
use ferrum_kernel::tenant::NewTenant;
use ferrum_kernel::{Ctx, Db, DbConfig, Doc, Error, Filter, HookRegistry, Kernel, KernelSettings, ListQuery};
use ferrum_meta::Catalog;
use serde_json::{Value, json};
use std::sync::Arc;
use uuid::Uuid;

fn obj(v: Value) -> Doc {
    v.as_object().cloned().unwrap()
}

fn uid(v: &Value) -> Uuid {
    v.as_str().unwrap_or_else(|| panic!("not an id: {v}")).parse().unwrap()
}

fn dec(v: &Value) -> f64 {
    v.as_str().map(|s| s.parse().unwrap()).or_else(|| v.as_f64()).unwrap_or_else(|| panic!("not a number: {v}"))
}

/// A call to Property's endpoints as `who` (`/api/app/property/{path}`).
async fn call(k: &Kernel, who: Option<&Ctx>, method: &str, path: &str, body: Value) -> Result<Value, Error> {
    let site = k.hooks.site("property").expect("property's endpoints").clone();
    let req = ferrum_kernel::SiteRequest {
        tenant: who.map(|c| c.tenant).unwrap_or_default(),
        user: who.cloned(),
        method: method.into(),
        path: format!("/{path}"),
        query: Default::default(),
        headers: Default::default(),
        body: if body.is_null() { vec![] } else { body.to_string().into_bytes() },
        client_ip: "203.0.113.9".into(),
    };
    let r = site.handle(k, req).await?;
    Ok(serde_json::from_slice(&r.body).unwrap())
}

fn day(s: &str) -> chrono::NaiveDate {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
}

struct T {
    k: Arc<Kernel>,
    tenant: Uuid,
    admin: Ctx,
}

impl T {
    async fn insert(&self, ctx: &Ctx, entity: &str, v: Value) -> Doc {
        self.k.insert(ctx, entity, obj(v)).await.unwrap_or_else(|e| panic!("insert {entity}: {e:?}"))
    }
    async fn get(&self, entity: &str, id: Uuid) -> Doc {
        self.k.get(&self.admin, entity, id, false).await.unwrap_or_else(|e| panic!("get {entity}: {e:?}"))
    }
    async fn find(&self, entity: &str, filters: Value) -> Vec<Doc> {
        let q = ListQuery { filters: ferrum_kernel::query::parse_filters(&filters).unwrap(), limit: Some(50), ..Default::default() };
        self.k.list(&self.admin, entity, &q).await.unwrap().rows
    }
    async fn find_one(&self, entity: &str, filters: Value) -> Doc {
        let rows = self.find(entity, filters.clone()).await;
        assert_eq!(rows.len(), 1, "{entity} {filters}");
        rows[0].clone()
    }
    async fn user(&self, email: &str, roles: &[&str]) -> Ctx {
        let roles: Vec<String> = roles.iter().map(|r| r.to_string()).collect();
        let id = self.k.create_user(&self.admin, email, email, Some("Correct-Horse-9"), &roles, false).await.expect("user");
        let s = self.k.login("harbour", email, "Correct-Horse-9").await.expect("login");
        self.k.load_ctx(self.tenant, id, Some(s.token_version), "it".into()).await.expect("ctx")
    }
    async fn status(&self, unit: Uuid) -> String {
        self.get("Unit", unit).await["status"].as_str().unwrap().to_string()
    }
    /// The rent invoiced for a lease: (first day, last day, amount), in order.
    async fn periods(&self, lease: Uuid) -> Vec<(String, String, f64)> {
        let mut out: Vec<(String, String, f64)> = self
            .find("RentInvoice", json!({"lease": lease.to_string()}))
            .await
            .iter()
            .map(|r| (r["period_from"].as_str().unwrap().into(), r["period_to"].as_str().unwrap().into(), dec(&r["amount"])))
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }
}

fn refused<V: std::fmt::Debug>(r: Result<V, Error>, kind: &str, says: &str) {
    let msg = format!("{r:?}");
    let ok = matches!(
        (&r, kind),
        (Err(Error::Invalid(_) | Error::Validation(_)), "invalid") | (Err(Error::Forbidden(_)), "forbidden") | (Err(Error::NotFound(_)), "not found")
    );
    assert!(ok && msg.contains(says), "expected {kind} saying {says:?}, got {msg}");
}

#[test]
fn a_year_of_leases() {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().thread_stack_size(16 << 20).build().unwrap();
    rt.block_on(async { tokio::spawn(scenario()).await.unwrap() });
}

async fn scenario() {
    let url = std::env::var("FERRUM_PROPERTY_TEST_DATABASE_URL").unwrap_or_else(|_| "postgres://ferrum:ferrum@localhost:5432/ferrum_property_test".into());
    let db = Db::connect(DbConfig { url, max_connections: 16, ..Default::default() }).await.expect("connect");
    sqlx::raw_sql("drop schema if exists public cascade; create schema public;").execute(&db.pool).await.expect("reset");
    let mut packages = ferrum_domain::packages().expect("platform packages");
    packages.extend(ferrum_app_erp::packages().expect("ERP's packages"));
    packages.extend(ferrum_app_property::packages().expect("app packages"));
    let catalog = Catalog::from_packages(packages).expect("catalog");
    let mut hooks = HookRegistry::default();
    ferrum_domain::register(&mut hooks);
    ferrum_app_erp::register(&mut hooks);
    ferrum_app_property::register(&mut hooks);
    let settings = KernelSettings { secret_key: Some(SecretKey::new([7u8; 32])), ..Default::default() };
    let k = Kernel::boot(db, catalog, hooks, settings, None).await.expect("boot");
    ferrum_domain::maintenance(&k).await.expect("partitions");
    ferrum_app_erp::maintenance(&k).await.expect("partitions");
    let tenant = k
        .create_tenant(NewTenant {
            slug: "harbour".into(),
            name: "Harbour Estates".into(),
            admin_email: "admin@harbour.test".into(),
            admin_password: "Correct-Horse-9".into(),
            admin_name: None,
            packages: vec!["property".into()],
            settings: Some(json!({"fiscal_year_start_month": 4, "timezone": "Asia/Kolkata"})),
        })
        .await
        .expect("tenant");
    let session = k.login("harbour", "admin@harbour.test", "Correct-Horse-9").await.expect("login");
    let admin = k.load_ctx(tenant, session.user, Some(session.token_version), "it".into()).await.expect("ctx");
    let t = T { k: k.clone(), tenant, admin };
    let a = t.admin.clone();

    // ───────────── the books and the buildings ─────────────
    let inr = t.find_one("Currency", json!({"code": "INR"})).await;
    let company = uid(&t.insert(&a, "Company", json!({"company_name": "Harbour Estates", "abbr": "HE", "default_currency": inr["id"]})).await["id"]);
    let services = t.find_one("ItemGroup", json!({"group_name": "Services"})).await;
    let nos = t.find_one("Uom", json!({"uom_name": "Nos"})).await;
    let item = |code: &str| json!({"item_code": code, "item_name": code, "item_group": services["id"], "stock_uom": nos["id"], "is_stock_item": false});
    let rent = uid(&t.insert(&a, "Item", item("RENT")).await["id"]);
    let parking = uid(&t.insert(&a, "Item", item("PARKING")).await["id"]);
    let ana = uid(&t.insert(&a, "Customer", json!({"customer_name": "Ana Rao", "customer_type": "Individual"})).await["id"]);
    let bo = uid(&t.insert(&a, "Customer", json!({"customer_name": "Bo Lin", "customer_type": "Individual"})).await["id"]);
    let manager = t.user("manager@harbour.test", &["Property Manager"]).await;
    let fixer = t.user("fixer@harbour.test", &["Facilities"]).await;
    // No settings yet: upkeep has nowhere to invoice from, and does nothing.
    ferrum_app_property::maintenance_on(&k, day("2026-01-15")).await.expect("upkeep without settings");
    t.insert(&manager, "PropertySettings", json!({"company": company.to_string(), "rent_item": rent.to_string()})).await;
    let tower = uid(&t.insert(&manager, "Building", json!({"building_name": "Harbour View", "address": "1 Quay Street"})).await["id"]);
    let unit = |n: &str| json!({"unit_number": n, "building": tower.to_string(), "unit_type": "Residential", "area": "80"});
    let u101 = uid(&t.insert(&manager, "Unit", unit("101")).await["id"]);
    let u102 = uid(&t.insert(&manager, "Unit", unit("102")).await["id"]);
    let u103 = uid(&t.insert(&manager, "Unit", unit("103")).await["id"]);
    assert_eq!(t.status(u101).await, "Vacant");

    // ───────────── a lease, its terms, and the unit's days ─────────────
    let lease = |u: Uuid, who: Uuid, from: &str, to: &str, rent: &str, frequency: &str, charges: Value| json!({"unit": u.to_string(), "lessee": who.to_string(), "starts_on": from, "ends_on": to, "rent": rent, "frequency": frequency, "charges": charges});
    let with_parking = json!([{"charge": "Parking", "item": parking.to_string(), "amount": "2000"}]);
    refused(
        t.k.insert(&manager, "Lease", obj(lease(u101, ana, "2026-01-01", "2025-12-01", "30000", "Monthly", json!([])))).await,
        "invalid",
        "after the lease starts",
    );
    refused(t.k.insert(&manager, "Lease", obj(lease(u101, ana, "2026-01-01", "2026-06-30", "0", "Monthly", json!([])))).await, "invalid", "above zero");
    let first = uid(&t.insert(&manager, "Lease", lease(u101, ana, "2026-01-01", "2026-06-30", "30000", "Monthly", with_parking.clone())).await["id"]);
    // A draft may still change; it lets nothing yet.
    t.k.update(&manager, "Lease", first, obj(json!({"notes": "keys handed over"})), None).await.expect("a draft changes");
    assert_eq!(t.status(u101).await, "Vacant");
    t.k.run_action(&manager, "Lease", first, "activate", Value::Null).await.expect("activated");
    let l1 = t.get("Lease", first).await;
    assert_eq!((l1["status"].as_str(), l1["next_invoice_on"].as_str()), (Some("Active"), Some("2026-01-01")));
    assert_eq!(t.status(u101).await, "Let");
    refused(t.k.run_action(&manager, "Lease", first, "activate", Value::Null).await, "invalid", "not available");
    refused(t.k.update(&manager, "Lease", first, obj(json!({"rent": "25000"})), None).await, "invalid", "keeps its terms");
    refused(t.k.update(&manager, "Lease", first, obj(json!({"charges": []})), None).await, "invalid", "keeps its terms");
    t.k.update(&manager, "Lease", first, obj(json!({"notes": "pays on the 5th"})), None).await.expect("notes are not terms");
    // Another lease on some of the same days is refused when it would start; after them, it is not.
    let second = uid(&t.insert(&manager, "Lease", lease(u101, bo, "2026-06-01", "2026-12-31", "31000", "Monthly", json!([]))).await["id"]);
    refused(t.k.run_action(&manager, "Lease", second, "activate", Value::Null).await, "invalid", "let on some of these days");
    let first_number = l1["number"].as_str().unwrap().to_string();
    refused(t.k.run_action(&manager, "Lease", second, "activate", Value::Null).await, "invalid", &first_number);
    t.k.update(&manager, "Lease", second, obj(json!({"starts_on": "2026-07-01"})), None).await.expect("a draft moves");

    // ───────────── upkeep: each month invoiced once ─────────────
    ferrum_app_property::maintenance_on(&k, day("2026-03-15")).await.expect("upkeep");
    let want = |months: &[(&str, &str)], amount: f64| months.iter().map(|(f, l)| (f.to_string(), l.to_string(), amount)).collect::<Vec<_>>();
    let q1 = [("2026-01-01", "2026-01-31"), ("2026-02-01", "2026-02-28"), ("2026-03-01", "2026-03-31")];
    assert_eq!(t.periods(first).await, want(&q1, 32000.0), "rent and parking for each month begun");
    ferrum_app_property::maintenance_on(&k, day("2026-03-15")).await.expect("upkeep again");
    assert_eq!(t.periods(first).await.len(), 3, "a period is invoiced once");
    // Its next invoice wound back (a mistaken fix by hand), a period still is not invoiced again.
    let mut tx = t.k.db.tenant_tx(t.tenant).await.unwrap();
    sqlx::query("update e_lease set next_invoice_on = $2 where tenant_id = $1 and id = $3")
        .bind(t.tenant)
        .bind(day("2026-02-01"))
        .bind(first)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    ferrum_app_property::maintenance_on(&k, day("2026-03-15")).await.expect("upkeep after the wind-back");
    assert_eq!(t.periods(first).await.len(), 3, "February and March are not invoiced twice");
    let jan = t.find_one("RentInvoice", json!({"lease": first.to_string(), "period_from": "2026-01-01"})).await;
    let inv = t.get("SalesInvoice", uid(&jan["sales_invoice"])).await;
    assert_eq!(inv["customer"], ana.to_string());
    let mut lines: Vec<(String, f64)> =
        inv["items"].as_array().unwrap().iter().map(|l| (l["item"].as_str().unwrap().to_string(), dec(&l["price_list_rate"]))).collect();
    lines.sort_by(|x, y| x.1.total_cmp(&y.1));
    assert_eq!(lines, vec![(parking.to_string(), 2000.0), (rent.to_string(), 30000.0)]);
    assert_eq!(dec(&inv["grand_total"]), 32000.0);
    assert_eq!(t.get("Lease", first).await["next_invoice_on"], "2026-04-01");
    // Its last month invoiced, a lease still runs to its end.
    ferrum_app_property::maintenance_on(&k, day("2026-06-10")).await.expect("upkeep");
    assert_eq!(t.periods(first).await.len(), 6);
    assert_eq!(t.get("Lease", first).await["status"], "Active", "the lease runs to the 30th");
    assert_eq!(t.status(u101).await, "Let");
    ferrum_app_property::maintenance_on(&k, day("2026-07-01")).await.expect("upkeep");
    assert_eq!(t.get("Lease", first).await["status"], "Ended");
    assert_eq!(t.status(u101).await, "Vacant");
    assert_eq!(t.periods(first).await.len(), 6, "nothing after its end");
    t.k.run_action(&manager, "Lease", second, "activate", Value::Null).await.expect("the next lease, after the first");
    assert_eq!(t.status(u101).await, "Let");

    // ───────────── a quarterly lease, cut at its end ─────────────
    let quarterly = uid(&t.insert(&manager, "Lease", lease(u102, bo, "2026-01-01", "2026-05-15", "90000", "Quarterly", json!([]))).await["id"]);
    t.k.run_action(&manager, "Lease", quarterly, "activate", Value::Null).await.unwrap();
    ferrum_app_property::maintenance_on(&k, day("2026-06-01")).await.expect("upkeep");
    assert_eq!(t.periods(quarterly).await, want(&[("2026-01-01", "2026-03-31"), ("2026-04-01", "2026-05-15")], 90000.0));
    assert_eq!(t.get("Lease", quarterly).await["status"], "Ended");

    // ───────────── a lease ended early, invoiced to its last day ─────────────
    refused(t.k.run_action(&manager, "Lease", second, "terminate", json!({"date": "2026-06-30"})).await, "invalid", "from the lease's start to today");
    refused(t.k.run_action(&manager, "Lease", second, "terminate", json!({"date": "2099-01-01"})).await, "invalid", "from the lease's start to today");
    refused(t.k.run_action(&manager, "Lease", second, "terminate", Value::Null).await, "invalid", "last day");
    t.k.run_action(&manager, "Lease", second, "terminate", json!({"date": "2026-09-30"})).await.expect("ended early");
    assert_eq!(t.status(u101).await, "Vacant", "freed when it ends");
    ferrum_app_property::maintenance_on(&k, day("2026-12-01")).await.expect("upkeep");
    let to_september = [("2026-07-01", "2026-07-31"), ("2026-08-01", "2026-08-31"), ("2026-09-01", "2026-09-30")];
    assert_eq!(t.periods(second).await, want(&to_september, 31000.0), "nothing after the day it ended");
    assert_eq!(t.get("Lease", second).await["status"], "Terminated");
    // Its days stay held: a lease over them is refused.
    let over = uid(&t.insert(&manager, "Lease", lease(u101, ana, "2026-09-01", "2026-12-31", "30000", "Monthly", json!([]))).await["id"]);
    refused(t.k.run_action(&manager, "Lease", over, "activate", Value::Null).await, "invalid", "let on some of these days");
    // The days after it ended early are free.
    let after = uid(&t.insert(&manager, "Lease", lease(u101, ana, "2026-10-01", "2026-12-31", "30000", "Monthly", json!([]))).await["id"]);
    t.k.run_action(&manager, "Lease", after, "activate", Value::Null).await.expect("after the early end");
    assert_eq!(t.status(u101).await, "Let");

    // ───────────── maintenance ─────────────
    let leak = uid(&t.insert(&fixer, "RepairRequest", json!({"unit": u103.to_string(), "summary": "Burst pipe", "priority": "Urgent"})).await["id"]);
    assert_eq!(t.status(u103).await, "Under Maintenance");
    let tap = uid(&t.insert(&fixer, "RepairRequest", json!({"unit": u103.to_string(), "summary": "Dripping tap"})).await["id"]);
    let wiring = uid(&t.insert(&fixer, "RepairRequest", json!({"unit": u103.to_string(), "summary": "Loose wiring", "priority": "Urgent"})).await["id"]);
    t.k.update(&fixer, "RepairRequest", leak, obj(json!({"status": "Done"})), None).await.unwrap();
    assert!(t.get("RepairRequest", leak).await["done_on"].is_string(), "the day it was done is kept");
    assert_eq!(t.status(u103).await, "Under Maintenance", "another urgent request is open");
    t.k.update(&fixer, "RepairRequest", wiring, obj(json!({"status": "In Progress"})), None).await.unwrap();
    assert_eq!(t.status(u103).await, "Under Maintenance");
    t.k.update(&fixer, "RepairRequest", wiring, obj(json!({"status": "Done"})), None).await.unwrap();
    assert_eq!(t.status(u103).await, "Vacant", "back in service; a normal request does not hold it");
    assert!(t.get("RepairRequest", tap).await["done_on"].is_null());
    // A let unit goes back to let; a request moved to another unit takes the hold with it.
    let lease3 = uid(&t.insert(&manager, "Lease", lease(u103, ana, "2026-10-01", "2027-09-30", "28000", "Monthly", json!([]))).await["id"]);
    t.k.run_action(&manager, "Lease", lease3, "activate", Value::Null).await.unwrap();
    let boiler = uid(&t.insert(&fixer, "RepairRequest", json!({"unit": u103.to_string(), "summary": "No hot water", "priority": "Urgent"})).await["id"]);
    assert_eq!(t.status(u103).await, "Under Maintenance");
    t.k.update(&fixer, "RepairRequest", boiler, obj(json!({"unit": u102.to_string()})), None).await.unwrap();
    assert_eq!((t.status(u103).await, t.status(u102).await), ("Let".to_string(), "Under Maintenance".to_string()));
    t.k.update(&fixer, "RepairRequest", boiler, obj(json!({"status": "Done"})), None).await.unwrap();
    assert_eq!(t.status(u102).await, "Vacant");

    // ───────────── who may do what ─────────────
    refused(t.k.insert(&fixer, "Lease", obj(lease(u102, bo, "2027-01-01", "2027-12-31", "30000", "Monthly", json!([])))).await, "forbidden", "");
    refused(t.k.run_action(&fixer, "Lease", lease3, "terminate", json!({"date": "2026-10-31"})).await, "forbidden", "");
    refused(t.k.insert(&fixer, "Unit", obj(unit("104"))).await, "forbidden", "");
    t.k.update(&manager, "Unit", u102, obj(json!({"status": "Let"})), None).await.expect("a unit's other fields change");
    assert_eq!(t.status(u102).await, "Vacant", "its status is what holds it, not typed in");
    refused(
        t.k.insert(&manager, "RentInvoice", obj(json!({"lease": first.to_string(), "period_from": "2026-07-01", "period_to": "2026-07-31"}))).await,
        "forbidden",
        "",
    );

    // ───────────── the portal's "Your home" ─────────────
    t.k.update(&a, "Customer", ana, obj(json!({"email": "ana@harbour.test"})), None).await.unwrap();
    let portal = |email: &'static str| {
        let k = k.clone();
        let a = a.clone();
        async move {
            let id = k.create_user(&a, email, "Lessee", None, &[], false).await.unwrap();
            k.set_user_kind(&a, id, "portal").await.unwrap();
            k.load_ctx(a.tenant, id, None, "it".into()).await.unwrap()
        }
    };
    let ana_p = portal("Ana@Harbour.test").await;
    assert!(ana_p.roles.is_empty() && t.k.list(&ana_p, "Lease", &ListQuery::default()).await.is_err(), "no rights of her own");
    let home = call(&k, Some(&ana_p), "GET", "portal/home", Value::Null).await.unwrap();
    let mut numbers: Vec<(String, String)> =
        home["leases"].as_array().unwrap().iter().map(|l| (l["unit_number"].as_str().unwrap().into(), l["status"].as_str().unwrap().into())).collect();
    numbers.sort();
    let want_leases: Vec<(String, String)> =
        [("101", "Active"), ("101", "Ended"), ("103", "Active")].iter().map(|(u, s)| (u.to_string(), s.to_string())).collect();
    assert_eq!(numbers, want_leases, "hers, past their draft; Bo's are not here");
    let ended = home["leases"].as_array().unwrap().iter().find(|l| l["status"] == "Ended").unwrap();
    assert_eq!((ended["building"].as_str(), ended["invoices"].as_array().unwrap().len()), (Some("Harbour View"), 6));
    assert!(ended["invoices"].as_array().unwrap().iter().all(|i| i["invoice"].is_null() && i["issued"] == false), "drafts are not hers to open yet");
    // Once accounts issue one, it opens on Your invoices.
    let q = ListQuery { filters: vec![Filter::eq("lease", ended["id"].as_str().unwrap())], ..Default::default() };
    let first_rent = t.k.list(&a, "RentInvoice", &q).await.unwrap().rows.remove(0);
    let issued = uid(&first_rent["sales_invoice"]);
    t.k.submit(&a, "SalesInvoice", issued, None).await.expect("accounts issue the rent");
    let home = call(&k, Some(&ana_p), "GET", "portal/home", Value::Null).await.unwrap();
    let ended = home["leases"].as_array().unwrap().iter().find(|l| l["status"] == "Ended").unwrap().clone();
    let open: Vec<&Value> = ended["invoices"].as_array().unwrap().iter().filter(|i| i["invoice"].is_string()).collect();
    assert_eq!(
        (open.len(), open[0]["invoice"].as_str(), open[0]["issued"].as_bool()),
        (1, Some(issued.to_string().as_str()), Some(true)),
        "the issued one opens on Your invoices"
    );
    // A repair in a unit she rents now, and none in another.
    let asked =
        call(&k, Some(&ana_p), "POST", "portal/requests", json!({"unit": u103.to_string(), "summary": "Window stuck", "priority": "Low"})).await.unwrap();
    let req = t.get("RepairRequest", uid(&asked["id"])).await;
    assert_eq!(
        (req["reported_by"].as_str(), req["unit"].as_str(), req["status"].as_str()),
        (Some("ana@harbour.test"), Some(u103.to_string().as_str()), Some("Open"))
    );
    refused(
        call(&k, Some(&ana_p), "POST", "portal/requests", json!({"unit": u102.to_string(), "summary": "Not mine"})).await,
        "invalid",
        "not a unit you rent now",
    );
    refused(
        call(&k, Some(&ana_p), "POST", "portal/requests", json!({"unit": u103.to_string(), "summary": "x", "priority": "Emergency"})).await,
        "invalid",
        "choose Low",
    );
    refused(call(&k, Some(&ana_p), "POST", "portal/requests", json!({"unit": u103.to_string(), "summary": "  "})).await, "invalid", "summary");
    let urgent =
        call(&k, Some(&ana_p), "POST", "portal/requests", json!({"unit": u103.to_string(), "summary": "Gas smell", "priority": "Urgent"})).await.unwrap();
    assert_eq!(t.status(u103).await, "Under Maintenance", "an urgent request from the portal holds the unit too");
    t.k.update(&fixer, "RepairRequest", uid(&urgent["id"]), obj(json!({"status": "Done"})), None).await.unwrap();
    let home = call(&k, Some(&ana_p), "GET", "portal/home", Value::Null).await.unwrap();
    let mut asked: Vec<(String, String)> =
        home["requests"].as_array().unwrap().iter().map(|r| (r["summary"].as_str().unwrap().into(), r["status"].as_str().unwrap().into())).collect();
    asked.sort();
    assert_eq!(asked, [("Gas smell".to_string(), "Done".to_string()), ("Window stuck".to_string(), "Open".to_string())], "only what she asked for");
    // Someone with no lease sees nothing; nobody signed in, nothing at all.
    let stranger = portal("cy@elsewhere.test").await;
    let none = call(&k, Some(&stranger), "GET", "portal/home", Value::Null).await.unwrap();
    assert!(none["leases"].as_array().unwrap().is_empty() && none["requests"].as_array().unwrap().is_empty());
    refused(
        call(&k, Some(&stranger), "POST", "portal/requests", json!({"unit": u103.to_string(), "summary": "Let me in"})).await,
        "invalid",
        "not a unit you rent now",
    );
    assert!(matches!(call(&k, None, "GET", "portal/home", Value::Null).await, Err(Error::Unauthorized(_))));
    // Her lease of 103 ended early, she asks for no more repairs there.
    t.k.run_action(&manager, "Lease", lease3, "terminate", json!({"date": "2026-10-31"})).await.unwrap();
    refused(
        call(&k, Some(&ana_p), "POST", "portal/requests", json!({"unit": u103.to_string(), "summary": "One more"})).await,
        "invalid",
        "not a unit you rent now",
    );
}
