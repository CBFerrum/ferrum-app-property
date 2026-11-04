# Changelog

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
