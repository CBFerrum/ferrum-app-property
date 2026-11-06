# Changelog

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
