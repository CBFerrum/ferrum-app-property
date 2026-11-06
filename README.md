# Property

Property and facilities for [Ferrum](https://github.com/CBFerrum/Ferrum):
- buildings and their units;
- leases, with rent and other charges invoiced each period through ERP;
- inspections, and maintenance requests;
- the portal's pages for lessees and owners.

| | |
|---|---|
| 🏢 App | `property`, native, in the server |
| 🧩 Package | `property` (needs ERP's selling and accounting) |
| 🖥️ Interface | its own, at `/property/`: the units by building, the leases ending soon and the open requests, and the records; on the portal, "Your home" and "Properties you own" |
| 🔐 Roles | Property Manager, Facilities; lessees and owners on the portal |
| 📜 Licence | AGPL-3.0-only |

## Highlights

- **One lease to a unit's days.** Activating a lease lets its unit. A second lease over any of the
  same days is refused, naming the lease that holds them. An ended or terminated lease keeps the
  days it ran.
- **Terms are kept.** A draft may change. Once active, a lease's unit, lessee, dates, rent,
  frequency, charges, rent reviews, deposit and notice stay as agreed: end it and make another.
  The rent may still be reviewed, for periods not yet invoiced.
- **Rent invoiced each period, once.** Upkeep invoices each period of an active lease, monthly,
  quarterly or yearly, from its start up to today. Each invoice is a Sales Invoice in ERP to the
  lessee, with the rent and each charge (parking, service) on its own line. A period is invoiced
  once however often upkeep runs, and the last period is cut at the lease's end.
- **Part periods.** A period cut short (at the lease's end, ended early, or on notice) owes its
  days' share of the rent and charges, "45 of 91 days" on the invoice.
- **Rent reviews.** A lease lists the rent from given days on: set in its draft, or reviewed while
  it runs, from the next period to invoice on. Each period is invoiced at the rent in effect on
  its first day.
- **Service charges.** A building's Service Charge Budget for a year lists its shared costs, and
  is shared among its units by floor area once in use. A lease that pays service charge is invoiced
  its unit's share with its rent, each period's line its days in each budget year at that year's
  rate (the next year's once its budget is in use, else the first's). Once the year is over,
  Reconcile takes what was spent and gives each such lease its share of the difference for its days
  within the year: an invoice for more, or a credit note against its latest issued invoice of the
  year for less, noted when there is none.
- **Indexed rent.** A lease may follow a Rent Index (a consumer price index, with its value from
  each day on). On each anniversary of its start, its rent follows the index's rise since the year
  before, by its latest values on those days, within a cap, and never falling unless a floor says
  how far. The change is kept as a rent review that names the values.
- **Ending.** A lease runs to its last day, then upkeep ends it and frees the unit. Ended early by
  agreement, on a day from its start to today, it frees the unit at once and is invoiced up to
  that day.
- **Days given back.** A lease ended early, or by notice that runs out within a period already
  invoiced, gives back what was invoiced for the days after its new last day:
  - an issued invoice gets a draft credit note against it, for those days' share of each line;
  - a draft invoice is made again for the days before, or goes with its period when none are left.

  A period keeps the last day it is billed for and its credit note.
- **Notice.** Either side gives notice, as the lease's notice days say. The lease then ends when
  the notice runs out: it is invoiced to that day, upkeep ends it, and the days after are free for
  another lease. Notice that would run past the lease's end is refused, since it runs to its end anyway.
- **Late fees.** With a late-fee item in the settings, an issued rent invoice still owed past its
  due day and the days allowed is charged a late fee once: a fixed fee and a share of what is
  owed, invoiced to the lessee.
- **Deposits through the books.** With a deposit account in the settings, activating a lease with
  a deposit makes a journal entry: the lessee owes it, and it is held. Once the lease is over, the
  deposit is given back. Anything kept goes to the deductions account, and the reason is recorded.
- **Maintenance.** An urgent request takes its unit out of service until every urgent request on it
  is done. The unit then goes back to let or vacant, as its leases say. A request keeps the day it
  was done.
- **Inspections.** Move-in, move-out and routine inspections, with a finding for each area.
- **Your home, on the portal.** A lessee signs in to the workspace's portal and sees their leases
  past their draft, each at its rent in effect today, the rent invoiced for each (opening on ERP's
  "Your invoices"), and the repairs
  they asked for. They ask for a repair only in a unit they rent now; an urgent one holds the unit
  as any other does.
- **Properties you own, on the portal.** A building names its owner's address. Signed in to the
  portal with it, the owner sees each of their buildings: its units, let or vacant, and the lease
  letting each now (to whom, until when, at its rent in effect today, reviews counted); the rent,
  charges and late fees invoiced and
  still owed, by the month their period starts, from issued invoices only; and its repairs, open or
  done in the last 90 days. Nobody else's buildings, and nothing of a building without their
  address.
- **Owners' statements.** For a building and some days: what was collected on its rent invoices for
  periods starting then (late fees in, credit notes out), less the management fee (the building's
  percentage) and the expenses charged to the owner (repairs done then, at their cost, once each,
  and lines added by hand). Billing the payout makes the owner's purchase invoice to the company,
  from the owner as ERP's supplier, for accounts to pay as ERP pays bills. The owner sees billed
  statements on the portal, and what is still to pay on them. An owner signs in to the portal once
  an administrator has made them a portal person (Administration, Users).
- **The workspace's day.** Upkeep, the last day of notice or an early end, and the day a repair was
  done all go by the workspace's own day, not UTC's.

## Develop

```bash
cargo test                       # needs FERRUM_PROPERTY_TEST_DATABASE_URL's database
cd ui && npm run build && npm run check
```

`tests/property_it.rs` walks a year at a block of flats:
- leases, their terms and the unit's days;
- rent and parking invoiced each month, once;
- a quarterly lease cut at its end, paying for its days, and a lease ended early;
- rent reviewed, notice that ends a lease with its last day's share, a late fee charged once, and
  a deposit held and given back less what was kept;
- days invoiced past an early end given back: a credit note for an issued quarter, a drafted one
  gone, and a drafted month made again for its days, twice;
- urgent requests holding a unit;
- who may do what;
- the portal: only her leases, repairs only where she rents now; an owner sees only her building,
  with the rent issued and what is still owed after a payment;
- an owner's statement: a payment, a fee of 10%, a repair's cost and a line by hand making the
  share; overlapping days refused; billing refused without the owner in the books or the payout
  item, then billed and kept; the statement on the owner's portal, and what is still to pay once
  the bill is issued.

## Not yet

- Service charges are shared by floor area only (no weightings or schedules), and the landlord
  bears vacant units' shares. A statement's expenses are repairs and lines by hand: ERP's bills are not read for
  them, and its fee is one percentage of what was collected.
- Paying rent on the portal (ERP's invoices page shows what is due).

## Changelog

| Version | Highlights |
|---|---|
| **0.7.5** | A period across two budget years bears each year's days at that year's rate. |
| **0.7.4** | Rebuilt with Ferrum-Frontend a64416a: a box whose default is 1 shows ticked; action dialogs say every refusal and ask once. |
| **0.7.3** | The portals show a lease's rent in effect today, its reviews and indexing counted. |
| **0.7.2** | Reconciled against what was invoiced, to the cent; index reviews say what held the rise back; Owners in the settings. |
| **0.7.1** | Rebuilt with Ferrum-Frontend a76295e: times on a person's own clock when they chose one. Today is the workspace's day, not UTC's. |
| **0.7.0** | Owners' statements: rent collected less the management fee and the expenses charged, billed to the company as the owner's share, and seen on the portal. |
| **0.6.0** | Service charge budgets: shared costs by floor area, invoiced with the rent, reconciled once the year is over. |
| **0.5.0** | Rent indexed each year: a Rent Index's values, a cap and a floor, kept as rent reviews. |
| **0.4.1** | Rebuilt with Ferrum-Frontend 24c3951: times on the workspace's clock; drafts headed as drafts. |
| **0.4.0** | A lease ended early or on notice gives back what was invoiced for the days after its last day: a credit note against an issued invoice, a draft made again. |
| **0.3.0** | Owners on the portal: their buildings' units, leases, rent invoiced and still owed, and repairs; dates by the workspace's day. |
| **0.2.1** | Rebuilt with Ferrum-Frontend bb60115: empty numbers read as the server reads them; settings open as their record. |
| **0.2.0** | Part periods paid for their days, rent reviews, notice, late fees, and deposits held and given back through the books. |
| **0.1.5** | Rebuilt with Ferrum-Frontend 00dea07: a workflow state's kept fields are read-only, its needed ones required. |
| **0.1.4** | Personal-data requests find the repairs a person reported. |
| **0.1.3** | Rebuilt with Ferrum-Frontend daeadda: actions say what they did. |
| **0.1.2** | Rebuilt with Ferrum-Frontend 80ff179: labels name their fields, and linked titles show their titles. |
| **0.1.1** | Rent opens on "Your invoices" once issued; a draft says it is not issued yet. |
| **0.1.0** | Buildings and units, leases invoiced each period through ERP, inspections, maintenance requests, and lessees' "Your home" on the portal (FERRUM-INDUSTRY-008). |

## License

AGPL-3.0-only. See [LICENSE](LICENSE).
