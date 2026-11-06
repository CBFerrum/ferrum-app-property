# Changelog

## 0.6.0

Service charges (FERRUM-INDUSTRY-008).

- **Service Charge Budget:** a building's shared costs for a year (its costs' total is the budget).
  Use It puts it in force, refused over another in use on the same days; it then keeps its
  building, year and costs.
- **Pays Service Charge** on a lease, kept as a term. Such a lease is invoiced its unit's share of
  the building's budget (by floor area) with its rent: each period's line is its days of the year,
  at the rate of the budget in use on its first day. Service Charge Item in the settings says what
  it is invoiced as. Ending a lease early gives back its service charge with its rent.
- **Reconcile**, once the year is over, takes what was spent. Each lease that paid service charge
  that year is given its share of the difference for its days within the year: an invoice for more,
  or a credit note against its latest issued invoice of the year for less, noted when there is
  none. The budget keeps each balance and what was made for it.

## 0.5.0

Rent indexed each year (FERRUM-INDUSTRY-008).

- **Rent Index:** a published index (a consumer price index) with its value from each day on,
  kept by a Property Manager.
- **Indexed To**, **Most A Year** (a cap) and **Least A Year** (a floor) on a lease, kept as its
  terms once it is active.
- **Upkeep:** on each anniversary of an indexed lease's start, before the period it holds for is
  invoiced, the rent follows the index's rise since the year before. It uses the index's latest
  values on those two days, at most the cap, and at least the floor; it never falls when there is
  no floor. The change is kept as a rent review naming the values ("Indexed to CPI: 106 over
  100"), so the periods from that day are invoiced at it. A review made by hand from the
  anniversary on stands instead.

## 0.4.1

Rebuilt with Ferrum-Frontend 24c3951: its pages show times on the workspace's clock, naming the zone when it is not the browser's, a draft numbered on submit is headed as a draft, and a record of an app that is not installed says so.

## 0.4.0

Days given back when a lease ends sooner (FERRUM-INDUSTRY-008).

- **End It Early** and **Give Notice** give back what was invoiced for the days after the lease's
  new last day. An issued rent invoice gets a draft credit note against it in ERP, for those days'
  share of each line (rent and each charge). A draft one is made again for the days before, or goes
  with its period when the whole period comes after the last day.
- A rent invoice keeps the last day it is billed for (Billed Until) and its credit note. The
  actions say how many periods they gave back.
- The owners' portal counts issued credit notes against the month they belong to.

## 0.3.0

Owners on the portal (FERRUM-INDUSTRY-008).

- **Properties you own:** a building names its owner's address (Owner). Signed in to the
  workspace's portal with it, the owner opens "Properties you own" at `/portal/owner/`
  (`GET /api/app/property/portal/owner`): each of their buildings with its units, let or vacant,
  and the lease letting each now (lessee, last day or the day notice runs out, rent); the rent,
  charges and late fees invoiced and still owed, by the month their period starts, from issued
  invoices only, the latest twelve months with any; and its repairs, open first, then those done
  in the last 90 days. Anyone else sees no building.
- **Personal data:** requests about a person find and blank a building's owner and manager
  addresses.
- **The workspace's day:** upkeep invoices and ends leases by each workspace's own day, and a
  lease's notice or early end ("to today") and the day a repair was done use it too, not UTC's.

## 0.2.1

Rebuilt with Ferrum-Frontend bb60115: a record page reads an empty number or check as the server does, so an action whose condition needs one shows, and a workspace's settings open as their one record.

## 0.2.0

Rent terms, notice, late fees and deposits (FERRUM-INDUSTRY-008).

- **Part periods:** a period cut short owes its days' share of the rent and charges, said on the
  invoice line ("45 of 91 days").
- **Rent reviews:** a lease's rent reviews (from a day on) are set in its draft, or added while it
  runs (Review the Rent), from the next period to invoice on and after those already made. Each
  period is invoiced at the rent in effect on its first day.
- **Notice:** a lease has its notice days. Give Notice, by the lessee or the landlord, ends it when
  the notice runs out. The lease is invoiced to that day and ended by upkeep, and the days after
  are free. Notice that would run past the lease's end is refused.
- **Late fees:** with a late-fee item, an issued rent invoice still owed past its due day and the
  days allowed (7 unless set) is charged a fixed fee and a share of what is owed, once, invoiced to
  the lessee.
- **Deposits:** with a deposit account (a liability), activating a lease with a deposit makes a
  journal entry for accounts to post: the lessee owes it, and it is held. Return the Deposit,
  once the lease is over, gives it back to the lessee's account less what was kept. What was kept
  goes to the deductions account (an income), with the reason.
- **Locked terms:** reviews, deposit and notice days are terms, kept once the lease is active.
- **The page:** it shows leases ending on notice.
- Rebuilt with Ferrum-Frontend 4a3681c.

## 0.1.5

Rebuilt with Ferrum-Frontend 00dea07: in a workflow state that names what may change, the other fields show read-only, and the fields the state needs show required.

## 0.1.4

Personal-data requests find the repairs a person reported; erasure takes their address off them, and the repairs stay (Ferrum's FERRUM-PRIVACY-002).

## 0.1.3

Rebuilt with Ferrum-Frontend daeadda: after an action runs, a record page shows what the action said it did rather than only that it is done.

## 0.1.2

Rebuilt with Ferrum-Frontend 80ff179: a field's label names its control for assistive technology, and a record whose title is a link shows that record's title, not its id.

## 0.1.1

From a browser walk of the lessees' portal.

- Rent invoiced opens on ERP's "Your invoices" once the invoice is issued; a draft says it is not issued yet, since "Your invoices" shows only issued ones (it linked to an invoice the lessee could not open).
- Rebuilt with Ferrum-Frontend's fields that name their controls, so the portal's forms are named for assistive technology.

## 0.1.0

Property and facilities (FERRUM-INDUSTRY-008).

- Buildings and units, each vacant, let or under maintenance as what holds it says.
- Leases: one to a unit's days, terms kept once active, ended at their end or early.
- Rent and other charges invoiced each period, once, through ERP to the lessee.
- Maintenance requests, an urgent one holding its unit until done; inspections with findings.
- Its own interface at `/property/`: the units by building, leases ending soon and open requests.
- The portal's "Your home" at `/portal/home/`: a lessee's leases and rent, and repairs asked for.
