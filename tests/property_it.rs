//! A year at a block of flats (FERRUM-INDUSTRY-008) against a real PostgreSQL
//! (`FERRUM_PROPERTY_TEST_DATABASE_URL`, default `postgres://ferrum:ferrum@localhost:5432/ferrum_property_test`;
//! reset first): a lease activated, letting its unit, and its terms kept; a second lease on the same
//! days refused; upkeep invoicing each month's rent and parking once through ERP, then ending the
//! lease and freeing the unit; a quarterly lease's last period cut at its end; a lease ended early,
//! invoiced to its last day; urgent requests taking a unit out of service until done; who may do
//! what; the portal's "Your home", where a lessee sees only their leases and asks for repairs only
//! in a unit they rent now; owners' statements, the rent collected less the management fee and the
//! expenses charged to them, billed to the company as their share; and "Properties you own", where an
//! owner sees only their buildings, with
//! the rent invoiced and still owed once issued.

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
    // Its last quarter, cut at its end, owes its 45 days of 91.
    let mut cut = want(&[("2026-01-01", "2026-03-31")], 90000.0);
    cut.extend(want(&[("2026-04-01", "2026-05-15")], 44505.49));
    assert_eq!(t.periods(quarterly).await, cut);
    let last_quarter = t.find_one("RentInvoice", json!({"lease": quarterly.to_string(), "period_from": "2026-04-01"})).await;
    let inv = t.get("SalesInvoice", uid(&last_quarter["sales_invoice"])).await;
    assert!(inv["items"][0]["description"].as_str().unwrap().ends_with("(45 of 91 days)"), "{}", inv["items"][0]);
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
    // The days after it ended early are free. Its rent is reviewed from 15 October, so the portals
    // show 30500 from then on.
    let mut terms_101 = lease(u101, ana, "2026-10-01", "2026-12-31", "30000", "Monthly", json!([]));
    terms_101["rent_reviews"] = json!([{"effective_from": "2026-10-15", "rent": "30500"}]);
    let after = uid(&t.insert(&manager, "Lease", terms_101).await["id"]);
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
    let let_101 = home["leases"].as_array().unwrap().iter().find(|l| l["unit_number"] == "101" && l["status"] == "Active").unwrap();
    assert_eq!(dec(&let_101["rent"]), 30500.0, "the rent in effect today, its review counted");
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

    // ───────────── rent reviews, notice, late fees and the deposit ─────────────
    let account = |name: &'static str| {
        let t = &t;
        async move { t.find_one("Account", json!({"account_name": name, "company": company.to_string()})).await["id"].clone() }
    };
    let held = t
        .insert(&a, "Account", json!({"account_name": "Security Deposits", "company": company.to_string(), "parent_account": account("Current Liabilities").await, "root_type": "Liability"}))
        .await["id"]
        .clone();
    let other_income = account("Other Income").await;
    let late_item = uid(&t.insert(&a, "Item", item("LATEFEE")).await["id"]);
    let settings = uid(&t.find_one("PropertySettings", json!({})).await["id"]);
    t.k.update(
        &manager,
        "PropertySettings",
        settings,
        obj(json!({"late_fee_item": late_item.to_string(), "late_fee_after_days": 5, "late_fee_amount": "500", "late_fee_percent": "2",
                   "deposit_account": held, "deposit_deductions_account": other_income})),
        None,
    )
    .await
    .expect("late fees and deposits");
    let u104 = uid(&t.insert(&manager, "Unit", unit("104")).await["id"]);
    let cy = uid(&t.insert(&a, "Customer", json!({"customer_name": "Cy Doe", "customer_type": "Individual"})).await["id"]);
    let terms = |reviews: Value| {
        let mut v = lease(u104, cy, "2026-08-01", "2027-07-31", "20000", "Monthly", json!([]));
        v.as_object_mut().unwrap().extend(obj(json!({"deposit": "40000", "notice_days": 30, "rent_reviews": reviews})));
        v
    };
    refused(t.k.insert(&manager, "Lease", obj(terms(json!([{"effective_from": "2026-07-01", "rent": "21000"}])))).await, "invalid", "after the lease starts");
    let fourth = uid(&t.insert(&manager, "Lease", terms(json!([{"effective_from": "2027-02-01", "rent": "21000"}]))).await["id"]);
    t.k.run_action(&manager, "Lease", fourth, "activate", Value::Null).await.expect("let to Cy");
    // The deposit is owed by the lessee and held, through the books.
    let l4 = t.get("Lease", fourth).await;
    let je = t.get("JournalEntry", uid(&l4["deposit_entry"])).await;
    let mut lines: Vec<(String, f64, f64, Option<String>)> = je["accounts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| (l["account"].as_str().unwrap().into(), dec(&l["debit"]), dec(&l["credit"]), l["customer"].as_str().map(str::to_string)))
        .collect();
    lines.sort_by(|x, y| x.1.total_cmp(&y.1));
    assert_eq!(lines[0], (held.as_str().unwrap().to_string(), 0.0, 40000.0, None), "held");
    assert_eq!((lines[1].1, lines[1].3.as_deref()), (40000.0, Some(cy.to_string().as_str())), "owed by Cy");
    // Reviews while it runs: from the next period to invoice on, after those made.
    refused(
        t.k.run_action(&manager, "Lease", fourth, "review_rent", json!({"date": "2026-07-15", "rent": "22000"})).await,
        "invalid",
        "next period to invoice",
    );
    refused(
        t.k.run_action(&manager, "Lease", fourth, "review_rent", json!({"date": "2027-01-01", "rent": "22000"})).await,
        "invalid",
        "after the reviews already made",
    );
    refused(t.k.run_action(&manager, "Lease", fourth, "review_rent", json!({"date": "2027-05-01", "rent": "0"})).await, "invalid", "above zero");
    t.k.run_action(&manager, "Lease", fourth, "review_rent", json!({"date": "2027-05-01", "rent": "22000", "note": "Market review"})).await.expect("reviewed");
    assert_eq!(t.get("Lease", fourth).await["rent_reviews"].as_array().unwrap().len(), 2);
    refused(t.k.update(&manager, "Lease", fourth, obj(json!({"notice_days": 10})), None).await, "invalid", "keeps its terms");
    ferrum_app_property::maintenance_on(&k, day("2026-10-15")).await.expect("upkeep");
    assert_eq!(t.periods(fourth).await, want(&[("2026-08-01", "2026-08-31"), ("2026-09-01", "2026-09-30"), ("2026-10-01", "2026-10-31")], 20000.0));
    // August issued and left unpaid: once past its due day and five more, a fee, once.
    let august = t.find_one("RentInvoice", json!({"lease": fourth.to_string(), "period_from": "2026-08-01"})).await;
    let august_invoice = uid(&august["sales_invoice"]);
    let issued_august = t.k.submit(&a, "SalesInvoice", august_invoice, None).await.expect("issued");
    let due = day(issued_august["due_date"].as_str().or(issued_august["posting_date"].as_str()).unwrap());
    ferrum_app_property::maintenance_on(&k, due + chrono::Duration::days(5)).await.expect("upkeep");
    assert!(t.get("RentInvoice", uid(&august["id"])).await["late_fee_invoice"].is_null(), "not late yet");
    ferrum_app_property::maintenance_on(&k, due + chrono::Duration::days(6)).await.expect("upkeep");
    let fee = t.get("RentInvoice", uid(&august["id"])).await["late_fee_invoice"].clone();
    let fee_invoice = t.get("SalesInvoice", uid(&fee)).await;
    assert_eq!((fee_invoice["customer"].as_str(), dec(&fee_invoice["grand_total"])), (Some(cy.to_string().as_str()), 900.0), "500 and 2% of 20000");
    ferrum_app_property::maintenance_on(&k, due + chrono::Duration::days(9)).await.expect("upkeep");
    assert_eq!(t.get("RentInvoice", uid(&august["id"])).await["late_fee_invoice"], fee, "once");
    // Notice from the lessee: the lease ends when it runs out, its last period cut there.
    refused(
        t.k.run_action(&manager, "Lease", fourth, "give_notice", json!({"date": "2026-07-01", "by": "Lessee"})).await,
        "invalid",
        "from the lease's start to today",
    );
    refused(t.k.run_action(&manager, "Lease", fourth, "give_notice", json!({"date": "2026-11-01", "by": "Agent"})).await, "invalid", "lessee or the landlord");
    let noticed = t.k.run_action(&manager, "Lease", fourth, "give_notice", json!({"date": "2026-11-01", "by": "Lessee"})).await.expect("notice");
    assert_eq!(noticed["leaving_on"], "2026-12-01", "thirty days on");
    refused(t.k.run_action(&manager, "Lease", fourth, "give_notice", json!({"date": "2026-11-02", "by": "Landlord"})).await, "invalid", "notice was given on");
    let overlap = uid(&t.insert(&manager, "Lease", lease(u104, ana, "2026-11-20", "2027-03-31", "20000", "Monthly", json!([]))).await["id"]);
    refused(t.k.run_action(&manager, "Lease", overlap, "activate", Value::Null).await, "invalid", "let on some of these days");
    ferrum_app_property::maintenance_on(&k, day("2026-12-15")).await.expect("upkeep");
    let l4 = t.get("Lease", fourth).await;
    assert_eq!((l4["status"].as_str(), l4["terminated_on"].as_str()), (Some("Terminated"), Some("2026-12-01")));
    let periods = t.periods(fourth).await;
    assert_eq!(periods.last().unwrap(), &("2026-12-01".to_string(), "2026-12-01".to_string(), 645.16), "one day of December's 31");
    assert_eq!(t.status(u104).await, "Vacant");
    let next = uid(&t.insert(&manager, "Lease", lease(u104, ana, "2026-12-02", "2027-03-31", "20000", "Monthly", json!([]))).await["id"]);
    t.k.run_action(&manager, "Lease", next, "activate", Value::Null).await.expect("the days after notice ran out are free");
    // The deposit given back, less what was kept and why.
    refused(t.k.run_action(&manager, "Lease", second, "return_deposit", json!({})).await, "invalid", "no deposit was held");
    refused(t.k.run_action(&manager, "Lease", fourth, "return_deposit", json!({"kept": "5000"})).await, "invalid", "says why");
    refused(t.k.run_action(&manager, "Lease", fourth, "return_deposit", json!({"kept": "50000", "why": "x"})).await, "invalid", "from nothing to the deposit");
    t.k.run_action(&manager, "Lease", fourth, "return_deposit", json!({"kept": "5000", "why": "A broken window"})).await.expect("given back");
    let l4 = t.get("Lease", fourth).await;
    let back = t.get("JournalEntry", uid(&l4["deposit_return_entry"])).await;
    let mut lines: Vec<(String, f64, f64)> =
        back["accounts"].as_array().unwrap().iter().map(|l| (l["account"].as_str().unwrap().into(), dec(&l["debit"]), dec(&l["credit"]))).collect();
    lines.sort_by(|x, y| (x.1, x.2).partial_cmp(&(y.1, y.2)).unwrap());
    assert_eq!(lines[2], (held.as_str().unwrap().to_string(), 40000.0, 0.0), "the deposit leaves what is held");
    assert_eq!((lines[0].0.as_str(), lines[0].2), (other_income.as_str().unwrap(), 5000.0), "what was kept");
    assert_eq!(lines[1].2, 35000.0, "the rest owed back to Cy");
    assert_eq!(dec(&l4["deposit_kept"]), 5000.0);
    refused(t.k.run_action(&manager, "Lease", fourth, "return_deposit", json!({})).await, "invalid", "given back already");

    // ───────────── the portal's "Properties you own" ─────────────
    t.k.update(&manager, "Building", tower, obj(json!({"owner_email": "Olu@Owners.test"})), None).await.expect("its owner named");
    let quay = uid(&t.insert(&manager, "Building", json!({"building_name": "Quay House", "owner_email": "someone@else.test"})).await["id"]);
    t.insert(&manager, "Unit", json!({"unit_number": "1", "building": quay.to_string()})).await;
    // Accounts issue August's late fee, and Ana pays 12000 of her issued rent.
    t.k.submit(&a, "SalesInvoice", uid(&fee), None).await.expect("the fee issued");
    let bank = t.get("Company", company).await["default_bank_account"].clone();
    let paid = t
        .insert(
            &a,
            "PaymentEntry",
            json!({"company": company.to_string(), "payment_type": "Receive", "customer": ana.to_string(), "bank_account": bank, "currency": inr["id"],
                   "paid_amount": "12000", "references": [{"invoice_type": "Sales Invoice", "invoice": issued.to_string(), "allocated_amount": "12000"}]}),
        )
        .await;
    t.k.submit(&a, "PaymentEntry", uid(&paid["id"]), None).await.expect("paid");
    let olu = portal("olu@owners.test").await;
    let mine = call(&k, Some(&olu), "GET", "portal/owner", Value::Null).await.unwrap();
    let properties = mine["properties"].as_array().unwrap();
    assert_eq!(properties.len(), 1, "only the building naming her: {mine}");
    let harbour = &properties[0];
    assert_eq!((harbour["name"].as_str(), harbour["address"].as_str()), (Some("Harbour View"), Some("1 Quay Street")));
    let units: Vec<(String, String, Option<String>)> = harbour["units"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| (u["unit_number"].as_str().unwrap().into(), u["status"].as_str().unwrap().into(), u["lease"]["lessee"].as_str().map(str::to_string)))
        .collect();
    let want_units: Vec<(String, String, Option<String>)> =
        [("101", "Let", Some("Ana Rao")), ("102", "Vacant", None), ("103", "Vacant", None), ("104", "Let", Some("Ana Rao"))]
            .iter()
            .map(|(u, s, l)| (u.to_string(), s.to_string(), l.map(str::to_string)))
            .collect();
    assert_eq!(units, want_units, "each unit with the lease letting it now");
    let now_101 = &harbour["units"][0]["lease"];
    assert_eq!(
        (now_101["starts_on"].as_str(), now_101["ends_on"].as_str(), dec(&now_101["rent"])),
        (Some("2026-10-01"), Some("2026-12-31"), 30500.0),
        "at its rent in effect today"
    );
    // Only issued invoices, a late fee with its period, by the month the period starts in, newest first.
    let paid_month = t.find_one("RentInvoice", json!({"sales_invoice": issued.to_string()})).await["period_from"].as_str().unwrap()[..7].to_string();
    let by_month: Vec<(String, f64, f64)> =
        harbour["rent"].as_array().unwrap().iter().map(|m| (m["month"].as_str().unwrap().into(), dec(&m["invoiced"]), dec(&m["owed"]))).collect();
    assert_eq!(by_month, vec![("2026-08".to_string(), 20900.0, 20900.0), (paid_month, 32000.0, 20000.0)], "rent and parking; rent and its fee");
    // Repairs open first, then those done lately, with their units.
    let repairs: Vec<(String, String, String)> = harbour["repairs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| (r["summary"].as_str().unwrap().into(), r["unit_number"].as_str().unwrap().into(), r["status"].as_str().unwrap().into()))
        .collect();
    assert_eq!(repairs.len(), 6, "{repairs:?}");
    let mut open: Vec<&str> = repairs[..2].iter().map(|r| r.0.as_str()).collect();
    open.sort();
    assert_eq!(open, ["Dripping tap", "Window stuck"]);
    assert!(repairs[2..].iter().all(|r| r.2 == "Done") && repairs.iter().any(|r| r.0 == "No hot water" && r.1 == "102"), "{repairs:?}");
    // A lessee, a stranger and nobody signed in own nothing here.
    for who in [&ana_p, &stranger] {
        let none = call(&k, Some(who), "GET", "portal/owner", Value::Null).await.unwrap();
        assert!(none["properties"].as_array().unwrap().is_empty(), "{none}");
    }
    assert!(matches!(call(&k, None, "GET", "portal/owner", Value::Null).await, Err(Error::Unauthorized(_))));

    // ───────────── the owner's statement ─────────────
    // The boiler's repair cost 800, charged to the owner; the company keeps 10% for managing.
    t.k.update(&manager, "RepairRequest", boiler, obj(json!({"cost": "800"})), None).await.expect("its cost");
    t.k.update(&manager, "Building", tower, obj(json!({"management_fee_percent": "10"})), None).await.unwrap();
    let statement = |from: &str, to: &str| json!({"building": tower.to_string(), "period_from": from, "period_to": to, "expenses": [{"description": "Gutters cleaned", "amount": "500"}]});
    let os = t.insert(&manager, "OwnerStatement", statement("2026-01-01", "2030-12-31")).await;
    assert_eq!(
        (dec(&os["collected"]), dec(&os["management_fee"]), dec(&os["expenses_total"]), dec(&os["payout"])),
        (12000.0, 1200.0, 1300.0, 9500.0),
        "Ana's 12000 paid, less 10% and the expenses: {os:?}"
    );
    let lines: Vec<(String, f64, bool)> = os["expenses"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| (l["description"].as_str().unwrap().to_string(), dec(&l["amount"]), l["repair"].is_string()))
        .collect();
    assert!(
        lines.contains(&("Gutters cleaned".to_string(), 500.0, false)) && lines.iter().any(|l| l.0.contains("No hot water") && l.1 == 800.0 && l.2),
        "one added by hand, and the repair: {lines:?}"
    );
    refused(t.k.insert(&manager, "OwnerStatement", obj(statement("2026-09-01", "2026-09-30"))).await, "invalid", "covers some of these days already");
    let os = uid(&os["id"]);
    refused(t.k.run_action(&manager, "OwnerStatement", os, "bill_owner", json!({})).await, "invalid", "names no owner in the books");
    let olu_books = uid(&t.insert(&a, "Supplier", json!({"supplier_name": "Olu Owner"})).await["id"]);
    t.k.update(&manager, "Building", tower, obj(json!({"owner_supplier": olu_books.to_string()})), None).await.unwrap();
    refused(t.k.run_action(&manager, "OwnerStatement", os, "bill_owner", json!({})).await, "invalid", "no Owner Payout Item");
    let payout_item = uid(&t.insert(&a, "Item", item("OWNER-PAYOUT")).await["id"]);
    let property_settings = uid(&t.find_one("PropertySettings", json!({})).await["id"]);
    t.k.update(&manager, "PropertySettings", property_settings, obj(json!({"owner_payout_item": payout_item.to_string()})), None).await.unwrap();
    let billed = t.k.run_action(&manager, "OwnerStatement", os, "bill_owner", json!({})).await.expect("billed");
    let bill = t.get("PurchaseInvoice", uid(&billed["purchase_invoice"]["id"])).await;
    assert_eq!(
        (bill["supplier"].as_str(), dec(&bill["grand_total"]), bill["docstatus"].as_i64()),
        (Some(olu_books.to_string().as_str()), 9500.0, Some(0)),
        "the owner's bill, for accounts to issue"
    );
    refused(t.k.update(&manager, "OwnerStatement", os, obj(json!({"period_to": "2031-12-31"})), None).await, "invalid", "kept as it was billed");
    // Its owner sees it on the portal; once accounts issue the bill, what is still to pay.
    let mine = call(&k, Some(&olu), "GET", "portal/owner", Value::Null).await.unwrap();
    let st = &mine["properties"][0]["statements"][0];
    assert_eq!((dec(&st["collected"]), dec(&st["payout"]), st["still_to_pay"].is_null()), (12000.0, 9500.0, true), "{st}");
    t.k.submit(&a, "PurchaseInvoice", uid(&bill["id"]), None).await.expect("issued");
    let mine = call(&k, Some(&olu), "GET", "portal/owner", Value::Null).await.unwrap();
    assert_eq!(dec(&mine["properties"][0]["statements"][0]["still_to_pay"]), 9500.0);

    // ───────────── days invoiced past an early end, given back ─────────────
    let u105 = uid(&t.insert(&manager, "Unit", unit("105")).await["id"]);
    let quarters = uid(&t.insert(&manager, "Lease", lease(u105, bo, "2026-07-01", "2027-06-30", "92000", "Quarterly", with_parking.clone())).await["id"]);
    t.k.run_action(&manager, "Lease", quarters, "activate", Value::Null).await.unwrap();
    ferrum_app_property::maintenance_on(&k, day("2026-10-05")).await.expect("upkeep");
    let quarter = |from: &'static str| {
        let t = &t;
        async move { t.find("RentInvoice", json!({"lease": quarters.to_string(), "period_from": from})).await }
    };
    let (q3, q4) = (quarter("2026-07-01").await.remove(0), quarter("2026-10-01").await.remove(0));
    let (q3_invoice, q4_invoice) = (uid(&q3["sales_invoice"]), uid(&q4["sales_invoice"]));
    t.k.submit(&a, "SalesInvoice", q3_invoice, None).await.expect("accounts issue the third quarter");
    // Ended on 31 August: September given back on the issued quarter, and the drafted one gone.
    let ended = t.k.run_action(&manager, "Lease", quarters, "terminate", json!({"date": "2026-08-31"})).await.expect("ended early");
    assert_eq!(ended["periods_credited"], 2);
    let q3 = t.get("RentInvoice", uid(&q3["id"])).await;
    assert_eq!(q3["billed_to"], "2026-08-31");
    let note = t.get("SalesInvoice", uid(&q3["credit_note"])).await;
    assert_eq!((note["is_return"].as_bool(), note["return_against"].as_str()), (Some(true), Some(q3_invoice.to_string().as_str())));
    let mut back: Vec<(String, f64, f64)> =
        note["items"].as_array().unwrap().iter().map(|l| (l["item"].as_str().unwrap().to_string(), dec(&l["qty"]), dec(&l["price_list_rate"]))).collect();
    back.sort_by(|x, y| x.2.total_cmp(&y.2));
    assert_eq!(back, vec![(parking.to_string(), -1.0, 652.17), (rent.to_string(), -1.0, 30000.0)], "30 of the quarter's 92 days of rent and parking");
    assert!(quarter("2026-10-01").await.is_empty(), "a drafted period wholly after the last day goes");
    assert!(t.k.get(&a, "SalesInvoice", q4_invoice, false).await.is_err(), "and its draft with it");
    // Notice that ends a lease within a drafted month makes that month again for its days, and an
    // earlier end later makes it again.
    let u106 = uid(&t.insert(&manager, "Unit", unit("106")).await["id"]);
    let mut monthly = lease(u106, cy, "2026-09-01", "2027-08-31", "31000", "Monthly", json!([]));
    monthly["notice_days"] = json!(14);
    let months = uid(&t.insert(&manager, "Lease", monthly).await["id"]);
    t.k.run_action(&manager, "Lease", months, "activate", Value::Null).await.unwrap();
    ferrum_app_property::maintenance_on(&k, day("2026-10-05")).await.expect("upkeep");
    let october = || {
        let t = &t;
        async move { t.find_one("RentInvoice", json!({"lease": months.to_string(), "period_from": "2026-10-01"})).await }
    };
    let october_invoice = uid(&october().await["sales_invoice"]);
    let noticed = t.k.run_action(&manager, "Lease", months, "give_notice", json!({"date": "2026-10-01", "by": "Lessee"})).await.expect("notice");
    assert_eq!((noticed["leaving_on"].as_str(), noticed["periods_credited"].as_i64()), (Some("2026-10-15"), Some(1)));
    let remade = |days: &'static str, amount: f64| {
        let t = &t;
        async move {
            let r = october().await;
            let inv = t.get("SalesInvoice", october_invoice).await;
            assert_eq!((dec(&r["amount"]), dec(&inv["items"][0]["price_list_rate"])), (amount, amount));
            assert!(inv["items"][0]["description"].as_str().unwrap().ends_with(days), "{}", inv["items"][0]);
            r["period_to"].as_str().unwrap().to_string()
        }
    };
    assert_eq!(remade("to 2026-10-15 (15 of 31 days)", 15000.0).await, "2026-10-15");
    t.k.run_action(&manager, "Lease", months, "terminate", json!({"date": "2026-10-10"})).await.expect("ended sooner");
    assert_eq!(remade("to 2026-10-10 (10 of 31 days)", 10000.0).await, "2026-10-10");
    assert_eq!(t.periods(months).await[0], ("2026-09-01".to_string(), "2026-09-30".to_string(), 31000.0), "September as it was");

    // ───────────── a rent indexed each year ─────────────
    let cpi = uid(
        &t.insert(
            &manager,
            "RentIndex",
            json!({"index_name": "CPI", "values": [{"month": "2026-01-01", "value": "100"}, {"month": "2027-01-01", "value": "106"}, {"month": "2028-01-01", "value": "102"}]}),
        )
        .await["id"],
    );
    let u107 = uid(&t.insert(&manager, "Unit", unit("107")).await["id"]);
    let mut terms = lease(u107, bo, "2026-01-01", "2028-12-31", "10000", "Monthly", json!([]));
    terms["rent_index"] = json!(cpi.to_string());
    terms["index_cap"] = json!("4");
    let indexed = uid(&t.insert(&manager, "Lease", terms).await["id"]);
    t.k.run_action(&manager, "Lease", indexed, "activate", Value::Null).await.unwrap();
    refused(t.k.update(&manager, "Lease", indexed, obj(json!({"index_cap": "10"})), None).await, "invalid", "keeps its terms");
    // A year on, its rent follows the index's 6%, held to its 4% cap; the year after, the index falls
    // and the rent does not.
    ferrum_app_property::maintenance_on(&k, day("2027-01-15")).await.expect("upkeep");
    ferrum_app_property::maintenance_on(&k, day("2028-01-15")).await.expect("upkeep");
    let reviews: Vec<(String, f64, String)> = t.get("Lease", indexed).await["rent_reviews"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| (r["effective_from"].as_str().unwrap().to_string(), dec(&r["rent"]), r["note"].as_str().unwrap_or_default().to_string()))
        .collect();
    assert_eq!(
        reviews,
        vec![
            ("2027-01-01".to_string(), 10400.0, "Indexed to CPI: 106 over 100, held to its cap of 4%".to_string()),
            ("2028-01-01".to_string(), 10400.0, "Indexed to CPI: 102 over 106, not lowered without a floor".to_string())
        ]
    );
    let periods = t.periods(indexed).await;
    let amount = |from: &str| periods.iter().find(|p| p.0 == from).map(|p| p.2);
    assert_eq!((amount("2026-12-01"), amount("2027-01-01"), amount("2028-01-01")), (Some(10000.0), Some(10400.0), Some(10400.0)), "{periods:?}");

    // ───────────── service charges, budgeted and reconciled ─────────────
    let service_item = uid(&t.insert(&a, "Item", item("SERVICE")).await["id"]);
    t.k.update(&manager, "PropertySettings", settings, obj(json!({"service_charge_item": service_item.to_string()})), None).await.expect("invoiced as");
    let works = uid(&t.insert(&manager, "Building", json!({"building_name": "Quay Works"})).await["id"]);
    let office = |n: &str, area: &str| json!({"unit_number": n, "building": works.to_string(), "unit_type": "Office", "area": area});
    let (wa, wb) = (uid(&t.insert(&manager, "Unit", office("A", "60")).await["id"]), uid(&t.insert(&manager, "Unit", office("B", "40")).await["id"]));
    let heads = json!([{"head": "Cleaning", "amount": "7300"}, {"head": "Security", "amount": "3650"}]);
    let budget = t
        .insert(&manager, "ServiceChargeBudget", json!({"building": works.to_string(), "year_from": "2025-01-01", "year_to": "2025-12-31", "heads": heads}))
        .await;
    assert_eq!(dec(&budget["budget"]), 10950.0, "its costs' total: 30 a day");
    let budget = uid(&budget["id"]);
    t.k.run_action(&manager, "ServiceChargeBudget", budget, "activate_budget", Value::Null).await.expect("in use");
    refused(
        t.k.update(&manager, "ServiceChargeBudget", budget, obj(json!({"heads": [{"head": "Cleaning", "amount": "1"}]})), None).await,
        "invalid",
        "keeps its building, year and costs",
    );
    let paying = |u: Uuid, who: Uuid, from: &str, to: &str| {
        let mut v = lease(u, who, from, to, "1000", "Monthly", json!([]));
        v["pays_service_charge"] = json!(true);
        v
    };
    let la = uid(&t.insert(&manager, "Lease", paying(wa, bo, "2025-01-01", "2025-12-31")).await["id"]);
    let lb = uid(&t.insert(&manager, "Lease", paying(wb, cy, "2025-07-01", "2025-12-31")).await["id"]);
    for l in [la, lb] {
        t.k.run_action(&manager, "Lease", l, "activate", Value::Null).await.unwrap();
    }
    ferrum_app_property::maintenance_on(&k, day("2026-01-05")).await.expect("upkeep");
    // Each month bears its days of the unit's share: 60% of 30 a day is 18, 40% is 12.
    let month = |l: Uuid, from: &'static str| {
        let t = &t;
        async move { dec(&t.find_one("RentInvoice", json!({"lease": l.to_string(), "period_from": from})).await["amount"]) }
    };
    assert_eq!((month(la, "2025-01-01").await, month(la, "2025-02-01").await, month(lb, "2025-07-01").await), (1558.0, 1504.0, 1372.0));
    // A's December issued; less was spent than budgeted.
    let december = t.find_one("RentInvoice", json!({"lease": la.to_string(), "period_from": "2025-12-01"})).await;
    let december = uid(&december["sales_invoice"]);
    t.k.submit(&a, "SalesInvoice", december, None).await.expect("issued");
    let done = t.k.run_action(&manager, "ServiceChargeBudget", budget, "reconcile", json!({"actual": "9855"})).await.expect("reconciled");
    assert_eq!(done["leases"], 2);
    let reconciled = t.get("ServiceChargeBudget", budget).await;
    assert_eq!((reconciled["status"].as_str(), dec(&reconciled["actual"])), (Some("Reconciled"), 9855.0));
    let balances: Vec<(Uuid, i64, f64, f64, f64)> = reconciled["balances"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| (uid(&r["lease"]), r["days"].as_i64().unwrap(), dec(&r["charged"]), dec(&r["due"]), dec(&r["difference"])))
        .collect();
    assert_eq!(balances, vec![(la, 365, 6570.0, 5913.0, -657.0), (lb, 184, 2208.0, 1987.2, -220.8)], "each its share of the difference, for its days");
    let credit = t.get("SalesInvoice", uid(&reconciled["balances"][0]["document"])).await;
    assert_eq!(
        (credit["is_return"].as_bool(), credit["return_against"].as_str()),
        (Some(true), Some(december.to_string().as_str())),
        "credited against its December"
    );
    assert_eq!(dec(&credit["items"][0]["price_list_rate"]), 657.0);
    assert!(reconciled["balances"][1]["note"].as_str().is_some_and(|n| n.starts_with("220.80 owed back")), "nothing issued to credit B's against");
    refused(t.k.run_action(&manager, "ServiceChargeBudget", budget, "reconcile", json!({"actual": "1"})).await, "invalid", "not available");
    // Each month's line is rounded on its own, so a lease is reconciled against what its invoices
    // bore: a third of 1000 over 2025 is 333.33 due, and its twelve months bore 333.34.
    let pier = uid(&t.insert(&manager, "Building", json!({"building_name": "Pier House"})).await["id"]);
    let room = |n: &str| json!({"unit_number": n, "building": pier.to_string(), "unit_type": "Office", "area": "10"});
    let p1 = uid(&t.insert(&manager, "Unit", room("P1")).await["id"]);
    for n in ["P2", "P3"] {
        t.insert(&manager, "Unit", room(n)).await;
    }
    let cleaning = json!([{"head": "Cleaning", "amount": "1000"}]);
    let pier_budget = uid(&t
        .insert(&manager, "ServiceChargeBudget", json!({"building": pier.to_string(), "year_from": "2025-01-01", "year_to": "2025-12-31", "heads": cleaning}))
        .await["id"]);
    t.k.run_action(&manager, "ServiceChargeBudget", pier_budget, "activate_budget", Value::Null).await.expect("in use");
    let lp = uid(&t.insert(&manager, "Lease", paying(p1, bo, "2025-01-01", "2025-12-31")).await["id"]);
    t.k.run_action(&manager, "Lease", lp, "activate", Value::Null).await.unwrap();
    ferrum_app_property::maintenance_on(&k, day("2026-01-05")).await.expect("upkeep");
    t.k.run_action(&manager, "ServiceChargeBudget", pier_budget, "reconcile", json!({"actual": "1000"})).await.expect("reconciled");
    let row = t.get("ServiceChargeBudget", pier_budget).await["balances"][0].clone();
    assert_eq!((dec(&row["charged"]), dec(&row["due"]), dec(&row["difference"])), (333.34, 333.33, -0.01), "what its invoices bore");
}
