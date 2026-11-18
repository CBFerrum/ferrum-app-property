//! Overlapping days under concurrent requests, against a real PostgreSQL
//! (`FERRUM_PROPERTY_TEST_DATABASE_URL`, default `postgres://ferrum:ferrum@localhost:5432/ferrum_property_test`;
//! reset first): fifty draft leases of one unit for the same year activated at once let it once;
//! fifty overlapping service charge budgets of one building put in use at once leave one in use;
//! fifty owner's statements of one building for the same days saved at once leave one. Each is its
//! own request on its own connection; the test holds their writes until all fifty have read what
//! they check, as when they arrive in the same instant.

use ferrum_kernel::tenant::NewTenant;
use ferrum_kernel::{Db, DbConfig, Doc, Error, HookRegistry, Kernel, KernelSettings, ListQuery};
use ferrum_meta::Catalog;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use uuid::Uuid;

const N: usize = 50;

fn obj(v: Value) -> Doc {
    v.as_object().cloned().unwrap()
}

fn uid(v: &Value) -> Uuid {
    v.as_str().unwrap_or_else(|| panic!("not an id: {v}")).parse().unwrap()
}

/// How many requests ended each way: `done`, or the refusal's code and words (numbers masked).
fn tally<T>(outcomes: &[Result<T, Error>]) -> BTreeMap<String, usize> {
    let mut seen = BTreeMap::new();
    for o in outcomes {
        let said = match o {
            Ok(_) => "done".to_string(),
            Err(e) => format!("{}: {}", e.code(), e.to_string().replace(|c: char| c.is_ascii_digit(), "#")),
        };
        *seen.entry(said).or_insert(0) += 1;
    }
    seen
}

/// Runs `n` calls of `f` at once, each on its own task and connection; their outcomes in order.
/// Writes to `table` are held until all `n` wait to make one, so that each has read what it checks
/// before any writes, as when they arrive in the same instant.
async fn at_once<T, F, Fut>(k: &Kernel, table: &str, n: usize, f: F) -> Vec<Result<T, Error>>
where
    T: Send + 'static,
    F: Fn(usize) -> Fut,
    Fut: std::future::Future<Output = Result<T, Error>> + Send + 'static,
{
    let mut hold = k.db.pool.begin().await.expect("a transaction");
    sqlx::query(sqlx::AssertSqlSafe(format!("lock table {table} in share mode"))).execute(&mut *hold).await.expect("writes held");
    let tasks: Vec<_> = (0..n).map(|i| tokio::spawn(f(i))).collect();
    let since = std::time::Instant::now();
    loop {
        let waiting: i64 = sqlx::query_scalar(
            "select count(*) from pg_locks where relation = to_regclass($1) and not granted \
             and database = (select oid from pg_database where datname = current_database())",
        )
        .bind(table)
        .fetch_one(&k.db.pool)
        .await
        .expect("waiting writes");
        if waiting >= n as i64 {
            break;
        }
        assert!(since.elapsed() < std::time::Duration::from_secs(60), "{waiting} of {n} came to write");
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    hold.commit().await.expect("writes let go");
    let mut out = Vec::with_capacity(n);
    for t in tasks {
        out.push(t.await.expect("a request's task"));
    }
    out
}

#[test]
fn fifty_requests_for_the_same_days_yield_one() {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().thread_stack_size(16 << 20).build().unwrap();
    rt.block_on(async { tokio::spawn(races()).await.unwrap() });
}

async fn races() {
    let url = std::env::var("FERRUM_PROPERTY_TEST_DATABASE_URL").unwrap_or_else(|_| "postgres://ferrum:ferrum@localhost:5432/ferrum_property_test".into());
    // A connection for each request, and some to spare.
    let db = Db::connect(DbConfig { url, max_connections: (N + 10) as u32, ..Default::default() }).await.expect("connect");
    sqlx::raw_sql("drop schema if exists public cascade; create schema public;").execute(&db.pool).await.expect("reset");
    let mut packages = ferrum_domain::packages().expect("platform packages");
    packages.extend(ferrum_app_erp::packages().expect("ERP's packages"));
    packages.extend(ferrum_app_property::packages().expect("app packages"));
    let catalog = Catalog::from_packages(packages).expect("catalog");
    let mut hooks = HookRegistry::default();
    ferrum_domain::register(&mut hooks);
    hooks.app("erp", &[], ferrum_app_erp::register);
    ferrum_app_property::register(&mut hooks);
    let k = Kernel::boot(db, catalog, hooks, KernelSettings::default(), None).await.expect("boot");
    ferrum_domain::maintenance(&k).await.expect("partitions");
    let tenant = k
        .create_tenant(NewTenant {
            slug: "harbour".into(),
            name: "Harbour Estates".into(),
            admin_email: "admin@harbour.test".into(),
            admin_password: "Correct-Horse-9".into(),
            admin_name: None,
            packages: vec!["property".into()],
            settings: Some(json!({"timezone": "Asia/Kolkata"})),
        })
        .await
        .expect("tenant");
    let session = k.login("harbour", "admin@harbour.test", "Correct-Horse-9").await.expect("login");
    let a = k.load_ctx(tenant, session.user, Some(session.token_version), "it".into()).await.expect("ctx");
    let manager_id =
        k.create_user(&a, "manager@harbour.test", "Manager", Some("Correct-Horse-9"), &["Property Manager".to_string()], false).await.expect("user");
    let s = k.login("harbour", "manager@harbour.test", "Correct-Horse-9").await.expect("login");
    let manager = k.load_ctx(tenant, manager_id, Some(s.token_version), "it".into()).await.expect("ctx");
    let id = |entity: &'static str, v: Value| {
        let (k, a) = (k.clone(), a.clone());
        async move { uid(&k.insert(&a, entity, obj(v)).await.unwrap_or_else(|e| panic!("insert {entity}: {e:?}"))["id"]) }
    };
    let one = |entity: &'static str, filters: Value| {
        let (k, a) = (k.clone(), a.clone());
        async move {
            let q = ListQuery { filters: ferrum_kernel::query::parse_filters(&filters).unwrap(), ..Default::default() };
            k.list(&a, entity, &q).await.unwrap().rows.remove(0)
        }
    };

    // The books, a building and a unit, and someone to let it to.
    let inr = one("Currency", json!({"code": "INR"})).await;
    let company = id("Company", json!({"company_name": "Harbour Estates", "abbr": "HE", "default_currency": inr["id"]})).await;
    let services = one("ItemGroup", json!({"group_name": "Services"})).await;
    let nos = one("Uom", json!({"uom_name": "Nos"})).await;
    let rent =
        id("Item", json!({"item_code": "RENT", "item_name": "RENT", "item_group": services["id"], "stock_uom": nos["id"], "is_stock_item": false})).await;
    id("PropertySettings", json!({"company": company.to_string(), "rent_item": rent.to_string()})).await;
    let tower = id("Building", json!({"building_name": "Harbour View"})).await;
    let unit = id("Unit", json!({"unit_number": "101", "building": tower.to_string(), "area": "80"})).await;
    let ana = id("Customer", json!({"customer_name": "Ana Rao", "customer_type": "Individual"})).await;

    // ───────────── fifty leases of one unit for 2026, activated at once ─────────────
    let mut leases = Vec::with_capacity(N);
    for _ in 0..N {
        let lease = json!({"unit": unit.to_string(), "lessee": ana.to_string(), "starts_on": "2026-01-01", "ends_on": "2026-12-31", "rent": "30000", "frequency": "Monthly"});
        leases.push(id("Lease", lease).await);
    }
    let let_out = tally(
        &at_once(&k, "e_lease", N, |i| {
            let (k, manager, lease) = (k.clone(), manager.clone(), leases[i]);
            async move { k.run_action(&manager, "Lease", lease, "activate", Value::Null).await }
        })
        .await,
    );

    // ───────────── fifty overlapping budgets of one building, put in use at once ─────────────
    let first_day = chrono::NaiveDate::from_ymd_opt(2027, 1, 1).unwrap();
    let mut budgets = Vec::with_capacity(N);
    for i in 0..N {
        let from = first_day + chrono::Duration::days(i as i64);
        let year = json!({"building": tower.to_string(), "year_from": from.to_string(), "year_to": (from + chrono::Duration::days(364)).to_string(),
                          "heads": [{"head": "Cleaning", "amount": "1200"}]});
        budgets.push(id("ServiceChargeBudget", year).await);
    }
    let in_use = tally(
        &at_once(&k, "e_service_charge_budget", N, |i| {
            let (k, manager, budget) = (k.clone(), manager.clone(), budgets[i]);
            async move { k.run_action(&manager, "ServiceChargeBudget", budget, "activate_budget", Value::Null).await }
        })
        .await,
    );

    // ───────────── fifty owner's statements of one building for one quarter, saved at once ─────────────
    let stated = tally(
        &at_once(&k, "e_owner_statement", N, |_| {
            let (k, manager) = (k.clone(), manager.clone());
            let statement = json!({"building": tower.to_string(), "period_from": "2026-01-01", "period_to": "2026-03-31"});
            async move { k.insert(&manager, "OwnerStatement", obj(statement)).await }
        })
        .await,
    );

    let done = |t: &BTreeMap<String, usize>| t.get("done").copied();
    assert_eq!(
        (done(&let_out), done(&in_use), done(&stated)),
        (Some(1), Some(1), Some(1)),
        "one each: leases activated {let_out:#?}, budgets put in use {in_use:#?}, statements saved {stated:#?}"
    );
    let mut tx = k.db.tenant_tx(tenant).await.unwrap();
    let kept: (i64, i64, i64) = sqlx::query_as(
        "select (select count(*) from e_lease where tenant_id = $1 and unit = $2 and status <> 'Draft'), \
                (select count(*) from e_service_charge_budget where tenant_id = $1 and building = $3 and status <> 'Draft'), \
                (select count(*) from e_owner_statement where tenant_id = $1 and building = $3)",
    )
    .bind(tenant)
    .bind(unit)
    .bind(tower)
    .fetch_one(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(kept, (1, 1, 1), "one kept each");
    assert_eq!(one("Unit", json!({"unit_number": "101"})).await["status"], "Let");
    // Everyone else is told the days are taken, in the app's words (naming the other) or the database's.
    let told = |seen: &BTreeMap<String, usize>, words: [&str; 2]| assert!(seen.keys().all(|s| s == "done" || words.contains(&s.as_str())), "{seen:#?}");
    told(&let_out, ["conflict: the unit is let on some of these days", "invalid: the unit is let on some of these days, under LSE-####-#####"]);
    told(
        &in_use,
        [
            "conflict: the building has a budget in use over some of these days",
            "invalid: the building has a budget in use from ####-##-## over some of these days",
        ],
    );
    told(
        &stated,
        [
            "conflict: another statement covers some of these days already",
            "validation_failed: validation failed: period_from: OS-####-##### covers some of these days already",
        ],
    );
}
