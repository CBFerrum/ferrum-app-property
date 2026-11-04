# Property

Property and facilities for [Ferrum](https://github.com/CBFerrum/Ferrum):
- buildings and their units;
- leases, with rent and other charges invoiced each period through ERP;
- inspections, and maintenance requests.

| | |
|---|---|
| 🏢 App | `property`, native, in the server |
| 🧩 Package | `property` (needs ERP's selling and accounting) |
| 🖥️ Interface | its own, at `/property/`: the units by building, the leases ending soon and the open requests, and the records |
| 🔐 Roles | Property Manager, Facilities; lessees on the portal |
| 📜 Licence | AGPL-3.0-only |

## Highlights

- **One lease to a unit's days.** Activating a lease lets its unit. A second lease over any of the
  same days is refused, naming the lease that holds them. An ended or terminated lease keeps the
  days it ran.
- **Terms are kept.** A draft may change. Once active, a lease's unit, lessee, dates, rent,
  frequency and charges stay as agreed: end it and make another.
- **Rent invoiced each period, once.** Upkeep invoices each period of an active lease, monthly,
  quarterly or yearly, from its start up to today. Each invoice is a Sales Invoice in ERP to the
  lessee, with the rent and each charge (parking, service) on its own line. A period is invoiced
  once however often upkeep runs, and the last period is cut at the lease's end.
- **Ending.** A lease runs to its last day, then upkeep ends it and frees the unit. Ended early, on a
  day from its start to today, it frees the unit at once and is invoiced up to that day.
- **Maintenance.** An urgent request takes its unit out of service until every urgent request on it
  is done. The unit then goes back to let or vacant, as its leases say. A request keeps the day it
  was done.
- **Inspections.** Move-in, move-out and routine inspections, with a finding for each area.
- **Your home, on the portal.** A lessee signs in to the workspace's portal and sees their leases
  past their draft, the rent invoiced for each (opening on ERP's "Your invoices"), and the repairs
  they asked for. They ask for a repair only in a unit they rent now; an urgent one holds the unit
  as any other does.

## Develop

```bash
cargo test                       # needs FERRUM_PROPERTY_TEST_DATABASE_URL's database
cd ui && npm run build && npm run check
```

`tests/property_it.rs` walks a year at a block of flats:
- leases, their terms and the unit's days;
- rent and parking invoiced each month, once;
- a quarterly lease cut at its end, and a lease ended early;
- urgent requests holding a unit;
- who may do what;
- the portal: only her leases, repairs only where she rents now.

## Not yet

- Rent for part of a period: the last, shorter period is charged in full. Rent reviews and
  escalations.
- Deposits held and returned through the books (the amount is recorded), and late fees.
- Ending a lease on a future day, with notice periods.
- Service-charge budgets shared across a building, and owners' statements for managed property.
- An owners' portal, and paying rent on the portal (ERP's invoices page shows what is due).

## Changelog

| Version | Highlights |
|---|---|
| **0.1.0** | Buildings and units, leases invoiced each period through ERP, inspections, maintenance requests, and lessees' "Your home" on the portal (FERRUM-INDUSTRY-008). |

## License

AGPL-3.0-only. See [LICENSE](LICENSE).
