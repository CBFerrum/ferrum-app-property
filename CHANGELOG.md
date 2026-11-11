# Changelog

## 0.8.1

The interface follows the upgraded design system, rebuilt with Ferrum-Frontend 0a47157. Text uses the kit's type roles (nothing under 12px) and controls its height tokens; loading shows skeletons shaped like what is coming; empty, no matches, denied, offline and failed are told apart; status colours come from stored state ids. The interface check now runs the UI lint against an empty baseline. The home is rebuilt on the kit's work-home pattern: one page title, the main action as a button, queues with their counts, status at its natural width; its other own pages follow the same anatomy.

## 0.8.0

Property accounting reconciled with the books (FEO-050).

- **Owners' expenses from ERP's bills.** A building names its owner's costs in the books (a cost center). A statement charges, each once, the bills submitted and posted in its days whose lines carry it (those lines' net), and the bills a repair of its units names (RepairRequest's new Bill: the whole bill's net, rather than the cost typed). Drafts are not charged; the owners' own payout bills never are.
- **Apportionment rules.** A Service Charge Budget has a basis: Floor Area (as before), Equal Shares, or Weighted Area (each unit's floor area by its new Weighting). A budget in use keeps its basis.
- **Spending from the books.** A budget names its shared costs in the books (a cost center of its own, never a building's owner's): Reconcile without a figure reads what was spent from the year's submitted bills charged to it.
- **Owner Reconciliation**, a report for Property Managers: each building's billed statements beside the ledger (expenses charged from bills under its cost center against what the books carry there in those days) and their payouts against the owner's bills, paid and still to pay.
- Names in Arabic and Hindi for the new fields and the report.

## 0.7.12

Rebuilt with Ferrum-Frontend c59c9a2: the names of its models, fields and manifest in Arabic and Hindi (240 each in ui/src/locales/names, which the platform serves to every page of the workspace, so a record's labels, the launcher and the bar read in the person's language), and the sentences its pages still showed in English (split around a link or a figure, built in code from a template or word by word) whole and translated (60 words each in ui/src/locales). Translated by the model, to be reviewed by native speakers.

## 0.7.11

Rebuilt with Ferrum-Frontend 155518c: the sentences its pages built in code (counts, dates, what a toast or a screen reader says) go through t() too, as whole sentences where code chose a word, and carry built-in Arabic and Hindi (56 words each in ui/src/locales, translated by the model and to be reviewed by native speakers).

## 0.7.10

The assistant knows what its records mean in its pages (contributes.shell.assistant): their states, what counts and where figures come from, with questions it suggests asking there.

## 0.7.9

Rebuilt with Ferrum-Frontend 5b22e7b: its own words carry built-in Arabic and Hindi (55 words each, ui/src/locales, translated by the model and to be reviewed by native speakers), fetched for the page's language only. The labels it keeps in objects, its dialogs, its page titles and the words it keeps in variables go through t() as well, and the names and sizes it shows as they are keep their order right to left. Its pages are drawn once their words are in, and its entry carries only the parts of the kit it uses.

## 0.7.8

Rebuilt with Ferrum-Frontend 6537d65: an action's link, code or token shown once is kept in a dialog to copy, and on a page that reads right to left, English left untranslated keeps its own order. Its own words go through the kit's t() (53 places), so a workspace's own words for its language reach them, and Admin, Interface words lists them under it (dist/words.json).

## 0.7.7

Rebuilt with Ferrum-Frontend d5d98f2: a workspace's own words for its interface (Admin, Interface words) reach its pages as they load; an attachment field that holds a picture shows it; and a value another record already has is said so, where the form said the record had been changed by someone else.

## 0.7.6

Rebuilt with Ferrum-Frontend 63280d3: right to left, its pages mirror as they read (icons that point along the line, margins and alignment by the line's start and end, arrow keys along it), and the records pages' words are in Arabic and Hindi.

## 0.7.5

A period across two budget years bears each year's days at that year's service charge rate: the
budget in use on its first day for its days in that year, the next year's for the days after once
its budget is in use (else the first year's, as before). A quarter from November to January under
budgets of 10 and 20 a day is charged 61 × 10 + 31 × 20, not 92 × 10. Reconciliation already took
each year's days by themselves.

## 0.7.4

Rebuilt with Ferrum-Frontend a64416a: a new record's box whose default is 1 shows ticked, as it is saved. An action's dialog says every refusal, the errors on what the action made (another record's rows) among them, and an action with parameters and a confirmation asks once.

## 0.7.3

Found walking it in a browser on 7 November: a lease indexed from 1,000 to 1,030 still showed 1,000
a month to its owner on the portal.

- "Your home" and "Properties you own" show a lease's rent in effect on the workspace's day: its
  latest review from that day or before (by hand or by indexing), else its starting rent, as each
  period is invoiced.

## 0.7.2

Found walking it in a browser on 6 November.

- A service-charge budget is reconciled against what each lease's invoices bore for the year:
  their service lines, less what credit notes took back, each for its period's days within the
  year. Each month's line is rounded on its own, so the budget's share worked out once could be a
  cent or so off what was billed (504.11 against 504.10 in the walk).
- A rent review made by an index says what held the rise back: "held to its cap of 3%", "held to
  its floor of 1%", or "not lowered without a floor".
- The settings show Owner Payout Item under its own heading, Owners.

## 0.7.1

Rebuilt with Ferrum-Frontend a76295e: its pages show times on a person's own clock when they chose one on their Account page, while today and due dates stay on the workspace's day. Its pages count today on the workspace's day, not UTC's, so the day's figures are right in the evening west of Greenwich and the early morning east of it.

## 0.7.0

Owners' statements (FERRUM-INDUSTRY-008).

- **Owner Statement:** for a building and some days, what was collected on its rent invoices for
  periods starting then (rent, charges and late fees paid; credit notes taken off), less the
  management fee (the building's Management Fee %) and the expenses charged to the owner: repairs
  of its units done then, at their new Cost, each on one statement only, and lines added by hand.
  A draft is worked out again whenever it is saved, and two statements of a building never share a
  day.
- **Bill The Payout:** the owner's share becomes their purchase invoice to the company, from the
  building's Owner In The Books (an ERP supplier), as the settings' Owner Payout Item, for accounts
  to issue and pay as ERP pays bills. The statement is then kept as billed. Nothing is billed when
  the fee and expenses come to more than was collected.
- **On the portal:** Properties you own lists each building's billed statements, with what is still
  to pay on the bill once accounts have issued it.
- A building's Owner says that its owner must be a portal person (Administration, Users) to sign in,
  which the walk of 6 November found unsaid.

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
