<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/assets/banner-dark.svg">
  <img alt="Ferrum Property: leases, rent, service charges and owners' statements in the books" src="docs/assets/banner-light.svg" width="100%">
</picture>

<br>

[![Version](https://img.shields.io/badge/version-0.8.2-0f766e?style=flat-square)](CHANGELOG.md)
[![Licence: AGPL-3.0](https://img.shields.io/badge/licence-AGPL--3.0-blue?style=flat-square)](LICENSE)
[![Ferrum platform](https://img.shields.io/badge/runs%20on-Ferrum-2f5bea?style=flat-square)](https://github.com/CBFerrum/Ferrum)
![Rust](https://img.shields.io/badge/Rust-server-b7410e?style=flat-square&logo=rust)
![PostgreSQL](https://img.shields.io/badge/PostgreSQL-data-336791?style=flat-square&logo=postgresql&logoColor=white)
![React](https://img.shields.io/badge/React%20%2B%20TypeScript-interface-087ea4?style=flat-square&logo=react&logoColor=white)
![Tests](https://img.shields.io/badge/tests-8%20Rust%20%2B%20UI%20suite-16a34a?style=flat-square)

### Rent invoiced on time, once, to the day, and owners told exactly what they are owed.

Ferrum Property is for landlords and property managers who let units to tenants and manage buildings for owners. Leases
are invoiced each period through Ferrum ERP. Deposits and service charges go through the books. Tenants and owners each
get a page on the workspace's portal.

[💡 Why](#-why-property) · [📸 Screenshots](#-screenshots) · [✨ Features](#-features) · [📊 Compare](#-how-it-compares) · [📍 Roadmap](#-roadmap) · [🚀 Install](#-install) · [🔧 Develop](#-develop)

</div>

## 💡 Why Property

- 📅 **Rent that bills itself, once.** Upkeep invoices each period of an active lease, monthly, quarterly or yearly. A period
  is invoiced once however often upkeep runs, and a period cut short owes its days' share ("45 of 91 days").
- 🔒 **Terms stay as agreed.** Once active, a lease keeps its unit, tenant, dates, rent, charges and deposit. Only the rent may
  be reviewed, for periods not yet invoiced. A unit has one lease on any day.
- 🧮 **The arithmetic is built in.** Rent reviews, indexation to a price index with a cap and a floor, part periods, late fees,
  and days given back by credit note when a lease ends early.
- 🏦 **Everything in the ledger.** Rent is a Sales Invoice, a deposit is a journal entry, an owner's payout is a Purchase
  Invoice. Owners' statements are reconciled to the ledger by a report.
- 🧑‍🤝‍🧑 **Portals for tenants and owners.** "Your home" and "Properties you own" on the workspace's portal, as configuration.
- 🔓 **Open source and self-hosted.** AGPL-3.0, on your own PostgreSQL, with no price per unit.
- 🌍 **Arabic and Hindi** words and names, with right-to-left layout.

## 🎯 At a glance

| | |
|---|---|
| **Version** | 0.8.2 |
| **Kind** | Its own interface at `/property/` (units by building, leases ending soon, open requests); the portal's "Your home" and "Properties you own"; every record in Ferrum's lists and forms |
| **What it adds** | 17 record types (9 documents, 7 child tables, 1 settings record), 2 roles, 1 report, 1 menu |
| **Depends on** | Ferrum platform `>=0.3.2, <0.4`; ERP `>=0.1` (installing Property installs ERP) |
| **Works with** | ERP (customers, suppliers, items, companies, accounts, cost centres, sales and purchase invoices, journal entries) |
| **Languages** | English, Arabic, Hindi (words and record names); right-to-left layout |
| **Tests** | 8 Rust tests (one walks a year at a block of flats on PostgreSQL) and a UI suite |
| **Licence** | AGPL-3.0-only |

## 📸 Screenshots

<div align="center">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/property-home-dark.webp"><img alt="Units by building" src="docs/assets/screens/property-home-light.webp" width="100%"></picture>
<p><b>🏢 Units by building</b><br><sub>Each building&#x27;s units with their status, the leases ending within 60 days and the open repair requests.</sub></p>
</div>

<table>
<tr>
<td width="50%" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/property-building-list-dark.webp"><img alt="Properties" src="docs/assets/screens/property-building-list-light.webp"></picture>
<p><b>🏘️ Properties</b><br><sub>Each property with its owner and manager.</sub></p>
</td>
<td width="50%" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/property-unit-list-dark.webp"><img alt="Units" src="docs/assets/screens/property-unit-list-light.webp"></picture>
<p><b>🚪 Units</b><br><sub>Units with their building, type, area and whether they are vacant, let or under maintenance.</sub></p>
</td>
</tr>
<tr>
<td width="50%" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/property-lease-list-dark.webp"><img alt="Leases" src="docs/assets/screens/property-lease-list-light.webp"></picture>
<p><b>📃 Leases</b><br><sub>Leases with their unit, tenant, term, rent and status.</sub></p>
</td>
<td width="50%" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/property-building-detail-dark.webp"><img alt="One property" src="docs/assets/screens/property-building-detail-light.webp"></picture>
<p><b>🧱 One property</b><br><sub>Read one property: address, owner, manager, management fee and cost centre.</sub></p>
</td>
</tr>
<tr>
<td width="50%" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/property-lease-detail-dark.webp"><img alt="One lease" src="docs/assets/screens/property-lease-detail-light.webp"></picture>
<p><b>🔑 One lease</b><br><sub>Read a lease: rent, frequency, deposit, notice, charges and rent reviews.</sub></p>
</td>
<td width="50%" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/property-repair-request-list-dark.webp"><img alt="Repair requests" src="docs/assets/screens/property-repair-request-list-light.webp"></picture>
<p><b>🔧 Repair requests</b><br><sub>Repairs by unit and priority, with who they are assigned to.</sub></p>
</td>
</tr>
</table>

### 📱 On a phone

<table>
<tr>
<td align="center" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/property-home-phone-dark.webp"><img alt="Units on a phone" src="docs/assets/screens/property-home-phone-light.webp" width="260"></picture>
<p><b>🏢 Units on a phone</b><br><sub>The home page on a narrow screen.</sub></p>
</td>
<td align="center" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/property-building-list-phone-dark.webp"><img alt="Properties on a phone" src="docs/assets/screens/property-building-list-phone-light.webp" width="260"></picture>
<p><b>🏘️ Properties on a phone</b><br><sub>Look up a property from the street.</sub></p>
</td>
</tr>
</table>


## ✨ Features

### 🏢 Buildings and units

- **Properties.** A building has a name, an address, a manager's email, an owner's email, the owner as ERP's supplier, a
  management fee percentage, and a cost centre for the owner's costs.
- **Units.** Each unit has a number, a building, a type (*Residential, Office, Retail, Industrial, Parking, Storage*), an area in
  square metres and a weighting. Units can be imported and exported.
- **Status.** A unit is *Vacant*, *Let* or *Under Maintenance*, kept for you from its leases and its urgent repairs.
- **Home page.** Units by building with their status, a queue of leases ending in the next 60 days (with the notice side when
  it ends on notice), and a queue of open requests that is flagged when any is urgent. A new property gets a first-run hint.

### 📃 Leases

- **A term and a rent.** A lease names a unit, a tenant (an ERP customer), start and end dates, a rent per period, a frequency
  (*Monthly, Quarterly, Yearly*), a deposit, notice days, other charges, rent reviews and notes.
- **Four states.** *Draft, Active, Ended, Terminated.* Activate It, End It Early, Give Notice, Review the Rent and Return the
  Deposit show only when they apply.
- **One lease to a unit's days.** Activating a lease lets its unit. A second lease over any of the same days is refused, naming
  the lease that holds them. An ended or terminated lease keeps the days it ran.
- **Terms are kept.** A draft may change. Once active, a lease's unit, tenant, dates, rent, frequency, charges, rent reviews,
  deposit and notice stay as agreed: end it and make another.
- **Ending.** A lease runs to its last day, then upkeep ends it and frees the unit. Ended early by agreement, on a day from its
  start to today, it frees the unit at once and is invoiced up to that day.
- **Notice.** Either side gives notice, as the lease's notice days say. The lease ends when the notice runs out: it is
  invoiced to that day, upkeep ends it, and the days after are free for another lease. Notice that would run past the lease's
  end is refused.
- **Numbered** `LSE-2026-00001`.

### 💸 Rent and charges

- **Invoiced each period, once.** Upkeep invoices each period from the lease's start up to today. Each invoice is a Sales Invoice
  in ERP to the tenant, with the rent and each other charge (parking, service) on its own line. The last period is cut at the
  lease's end.
- **Rent Invoice records.** One record for each period invoiced, with its dates, amount, invoice, late fee invoice and credit note.
- **Part periods.** A period cut short (at the lease's end, ended early, or on notice) owes its days' share of the rent and charges.
- **Rent reviews.** A lease lists the rent from given days on, set in the draft or reviewed while it runs from the next period to
  invoice. Each period is invoiced at the rent in effect on its first day.
- **Indexed rent.** A lease may follow a **Rent Index** (for example a consumer price index, with its value from each day on). On
  each anniversary of its start, the rent follows the index's rise since the year before, within a cap (*Most A Year*), and never
  falls unless a floor (*Least A Year*) says how far. The change is kept as a rent review that names the values.
- **Days given back.** A lease ended early, or by notice that runs out within a period already invoiced, gives back what was invoiced
  for the days after its new last day. An issued invoice gets a draft credit note for those days' share of each line. A draft
  invoice is made again for the days before, or goes with its period when none are left.
- **Late fees.** With a late-fee item in the settings, an issued rent invoice still owed past its due day and the days allowed is
  charged a late fee once: a fixed fee and a share of what is owed, invoiced to the tenant.
- **The workspace's day.** Upkeep, the last day of notice or an early end, and the day a repair was done go by the workspace's own
  day, not UTC's.

### 🏦 Deposits

- **Through the books.** With a deposit account in the settings, activating a lease with a deposit makes a journal entry: the
  tenant owes it, and it is held. Without one, the deposit is only recorded on the lease.
- **Returned.** Once the lease is over, Return the Deposit gives it back. Anything kept goes to the deductions account, and the
  reason is recorded.

### 🧾 Service charges

- **Budgets by year.** A **Service Charge Budget** for a building and a year lists its shared costs (cleaning, security, upkeep of
  the common parts). One budget per building and year.
- **Three ways to share.** By *Floor Area*, *Equal Shares*, or *Weighted Area* (floor area by each unit's weighting, so a shop on the
  street can bear more than its area). A budget in use keeps its basis.
- **Invoiced with the rent.** A lease that pays service charge is invoiced its unit's share with its rent. A period across two
  budget years bears each year's days at that year's rate.
- **Reconciled.** Once the year is over, Reconcile takes what was spent, given by hand or read from the submitted bills charged to the
  budget's own cost centre, and gives each paying lease its share of the difference for its days in the year: an invoice for more, or a
  credit note against its latest issued invoice of the year for less.
- **States.** A budget is *Draft*, *Active* or *Reconciled*.

### 🤝 Owners' statements

- **What an owner is owed.** For a building and some days: what was collected on its rent invoices for periods starting then (late
  fees in, credit notes out), less the management fee (the building's percentage) and the expenses charged to the owner.
- **Expenses, each once.** Submitted and posted bills in ERP whose lines carry the building's cost centre; the bill a repair
  of the building's units names (the whole bill, rather than the cost typed on the repair); repairs done then without a bill, at
  their cost; and lines added by hand. The owner's own payout bills are never charged.
- **Bill the payout.** Billing makes the owner's Purchase Invoice to the company, from the owner as ERP's supplier, for accounts
  payable as ERP pays bills. It is refused without the owner in the books or a payout item. A billed statement is kept as it is,
  and overlapping days are refused.
- **A building's owner's costs and a budget's shared costs have cost centres of their own,** never the same one.
- **Numbered** `OS-2026-00001`.

### 📊 Owner Reconciliation

- **A report that checks the books.** Each building's billed statements beside the ledger: what the statements charged from bills
  under its cost centre against what the ledger carries there in their days (a bill approved after its statement was billed shows as
  a difference), and the payouts against the owner's bills, paid and still to pay. For Property Managers and administrators.

### 🔍 Inspections and repairs

- **Inspections.** *Move In*, *Move Out* or *Routine*, for a unit and optionally a lease, with a date, an inspector, a status
  (*Scheduled*, *Done*) and a finding for each area (condition *Good, Fair, Poor, Damaged* and a note).
- **Repair requests.** A summary, details, priority (*Low, Normal, Urgent*), status (*Open, In Progress, Done*), who it is assigned to, a
  cost and the contractor's bill in ERP. The day it was done is kept.
- **Urgent holds the unit.** An urgent request takes its unit out of service until every urgent request on it is done. The unit then
  goes back to let or vacant, as its leases say.
- **Numbered** `MR-2026-00001`.

### 🧑‍🤝‍🧑 Portals

- **Your home** (`/portal/home/`). A tenant signs in to the workspace's portal and sees their leases past draft, each at the rent in
  effect today (reviews and indexing counted), the rent invoiced for each (opening on ERP's "Your invoices" once issued; a draft says it is
  not issued yet), and the repairs they asked for. They ask for a repair only in a unit they rent now, with how soon it is needed.
- **Properties you own** (`/portal/owner/`). A building names its owner's address. Signed in with it, the owner sees each of their
  buildings: its units, let or vacant, and the lease letting each now (to whom, until when, at its rent today); the rent, charges and late
  fees invoiced and still owed, by the month their period starts, from issued invoices only; its repairs, open or done in the last 90
  days; and billed statements with what is still to pay on them. Nobody else's buildings.
- **Set up by an administrator.** An owner or tenant is made a portal person in Administration, Users.

### 🔏 Personal data

- **Requests** find the repairs a person reported and the buildings naming a person as owner or manager, and erase those addresses.

### 🔐 Roles and permissions

| Who | Can |
|---|---|
| 🏠 Property Manager | Settings, buildings, units (import and export), leases, rent indexes, service charge budgets, owner statements; reads rent invoices; keeps inspections and repair requests; creates customers; reads suppliers, bills, cost centres, accounts and journal entries |
| 🛠️ Facilities | Reads buildings and units; keeps inspections and repair requests |
| 🙋 Tenants and owners on the portal | Only their own leases, repairs, buildings and statements, found by the address they sign in with |

### 🌍 Interface

- **Its own home** and portal pages; every record uses the shared list and form pages.
- **Languages.** English, Arabic and Hindi words, and Arabic and Hindi names for every record and field. A workspace can add its own
  words in Admin.
- **Right to left.** Pages mirror as they read.
- **Assistant.** It knows the states and where figures come from, and suggests "Which units are vacant?", "Which leases end in the
  next three months?", "Who is behind on rent?" and "Which repairs are still open?".

## 🧭 How it works

```mermaid
flowchart LR
  L["Lease (draft)"] -->|activate| A["Active: unit let, deposit held"]
  A -->|each period| RI["Rent invoice in ERP"]
  RI -->|owed past due| LF["Late fee, once"]
  A -->|notice or early end| E["Last day set: days given back"]
  E --> CN["Credit note or redrawn draft"]
  A -->|last day| EN["Ended: unit freed"]
  EN -->|return deposit| D["Deposit returned, less what was kept"]
```

Rent is invoiced once for each period from the lease's start up to today. Ending early or on notice cuts the last period and gives
back the days after.

```mermaid
flowchart TD
  RENT["Rent invoices collected"] --> OS["Owner statement"]
  BILLS["Bills under the building's cost centre"] --> OS
  REP["Repairs done"] --> OS
  OS -->|bill the payout| PI["Owner's purchase invoice"]
  OS --> REC["Owner Reconciliation report"]
  BUDGET["Service charge budget"] --> LEASE["Charge with the rent"]
  LEASE --> RECON["Year-end reconcile: invoice or credit note"]
```

An owner's statement is built from what was collected and what was charged in the ledger, then billed as a payout. The report
compares it with the books.

## 🧱 Data model

| Entity | What it is | Key fields or states |
|---|---|---|
| Property (`Building`) | A building | Address, manager, owner, owner in the books, management fee %, cost centre |
| Unit | A rentable unit | Number, building, type, area, weighting; *Vacant, Let, Under Maintenance* |
| Lease | A unit let for a term | Unit, tenant, dates, rent, frequency, deposit, notice days, charges, reviews, rent index, cap and floor; *Draft, Active, Ended, Terminated* |
| Charge, Rent Review | Rows of a lease | Charge, item, amount; from, rent, note |
| Rent Index | A published index | Name, values from each day on |
| Rent Invoice | One period of a lease, invoiced once | Lease, period, amount, invoice, late fee invoice, billed until, credit note |
| Service Charge Budget | A building's shared costs for a year | Building, year, basis, cost centre, costs, budget, spent, balances; *Draft, Active, Reconciled* |
| Owner Statement | What an owner is owed for some days | Building, period, collected, fee, expenses, payout, purchase invoice; *Draft, Billed* |
| Inspection | A look at a unit | Unit, lease, kind, date, inspector, findings; *Scheduled, Done* |
| Repair Request | Something to fix | Unit, summary, priority, assigned to, reported by, cost, bill; *Open, In Progress, Done* |
| Property Settings | Where rent is invoiced from | Company, rent item, late fee item, days, fixed fee and percentage, deposit accounts, service charge item, owner payout item |

**Roles:** Property Manager, Facilities; portal tenants and owners.

**Reports:** Owner Reconciliation.

**Print formats:** none yet.

**Scheduled upkeep:** invoice due periods, end leases, charge late fees, give back days, apply indexation, free units.

## 🔌 Integrations

| With | How | Needs |
|---|---|---|
| **Ferrum ERP** | Rent, charges, late fees, service-charge balances and credit notes are Sales Invoices; owners' payouts are Purchase Invoices from the owner as a supplier; deposits are journal entries; bills under cost centres feed statements and budgets | ERP installed with the app; a company and a rent item in Property Settings; accounts, items and cost centres as the features need |
| **Ferrum portal** | "Your home" and "Properties you own" | Portal enabled; tenants and owners made portal people |
| **Personal-data requests** | Repairs and building contact addresses | Nothing |

Not built in: online rent payment, rental applications, tenant screening, listings, e-signature, SMS and bulk notices.

## 📊 How it compares

Property is compared with **Yardi** (Voyager and Breeze), **AppFolio**, **Buildium** and **MRI Software**. There is no widely used open-source
property management suite; ERPNext has no such module. Those products are built for the leasing funnel, the tenant relationship
and trust accounting; this one is built around the lease, the period invoice and the ledger.

| Capability | Ferrum Property | Yardi | AppFolio | Buildium | MRI |
|---|:-:|:-:|:-:|:-:|:-:|
| Buildings and units with type, area and status | ✅ | ✅ | ✅ | ✅ | ✅ |
| Leases that cannot overlap on a unit | ✅ | ✅ | ✅ | ✅ | ✅ |
| Rent invoiced each period, automatically, with part periods | ✅ | ✅ | ✅ | ✅ | ✅ |
| Other charges invoiced with the rent | ✅ | ✅ | ✅ | ✅ | ✅ |
| Rent reviews and indexation with a cap and a floor | ✅ | ✅ | 🟡 | 🟡 | ✅ |
| Notice and early end, with the days given back by credit note | ✅ | ✅ | ✅ | ✅ | ✅ |
| Late fees | ✅ | ✅ | ✅ | ✅ | ✅ |
| Deposits held and returned through the ledger, with deductions | ✅ | ✅ | ✅ | ✅ | ✅ |
| Service charges (CAM): budget, share by area or weight, year-end reconcile | ✅ | ✅ | 🟡 | 🟡 | ✅ |
| Owners' statements with management fee, expenses and payout | ✅ | ✅ | ✅ | ✅ | ✅ |
| Owner portal | ✅ | ✅ | ✅ | ✅ | ✅ |
| Tenant portal with lease, rent invoiced and repairs | ✅ | ✅ | ✅ | ✅ | ✅ |
| Maintenance requests with priority | ✅ | ✅ | ✅ | ✅ | ✅ |
| Inspections: move in, move out, routine | 🟡 | ✅ | ✅ | ✅ | ✅ |
| Accounting, payables and bank in the same system | ✅ | ✅ | ✅ | ✅ | ✅ |
| Wider business apps (HR, CRM, projects) on the same data | ✅ | 🟡 | ❌ | ❌ | 🟡 |
| Trust accounting: owners' funds apart, reserves, fee types | 🟡 | ✅ | ✅ | ✅ | ✅ |
| Pay rent online, autopay, payment plans | ❌ | ✅ | ✅ | ✅ | ✅ |
| Rental applications, screening and approval into a lease | ❌ | ✅ | ✅ | ✅ | ✅ |
| Vacancy listings and showings | ❌ | ✅ | ✅ | ✅ | ✅ |
| Co-tenants, occupants, guarantors | ❌ | ✅ | ✅ | ✅ | ✅ |
| Lease renewals with offers and signed leases | ❌ | ✅ | ✅ | ✅ | ✅ |
| Work orders with vendors, quotes and approval limits | 🟡 | ✅ | ✅ | ✅ | ✅ |
| Preventive maintenance schedules | ❌ | ✅ | 🟡 | 🟡 | ✅ |
| Tenant ledger, arrears ageing and dunning notices | 🟡 | ✅ | ✅ | ✅ | ✅ |
| Rent roll, vacancy, delinquency and income reports | 🟡 | ✅ | ✅ | ✅ | ✅ |
| Commercial leases: percentage rent, options, critical dates | ❌ | ✅ | 🟡 | ❌ | ✅ |
| Bulk tenant notices and SMS | ❌ | ✅ | ✅ | ✅ | ✅ |
| Mobile app for managers and inspectors | ❌ | ✅ | ✅ | ✅ | ✅ |
| Self-hosted on your own servers | ✅ | 🟡 | ❌ | ❌ | 🟡 |
| Open source | ✅ | ❌ | ❌ | ❌ | ❌ |
| No licence fee per unit | ✅ | ❌ | ❌ | ❌ | ❌ |

Notes. **Ferrum 🟡:** inspections have findings by area with no photos; trust accounting is a cost centre per owner and a payout
purchase invoice, not owners' funds held apart; a repair's assignee is free text, with a cost and a bill link; a tenant's balance
is the outstanding amount of their invoices in ERP; reports are the home queues and one report. **Wider business apps** means other
Ferrum apps on the same platform. **Leaders:** 🟡 means a lighter feature, an add-on or a different edition of the product.

### 🏆 Where Property stands out

- 🧮 **Rent logic that shows its working.** Part periods ("45 of 91 days"), indexation that names the values it used, and days
  given back by credit note.
- 🏦 **One ledger.** Rent, deposits, service-charge balances and owners' payouts are ERP records. A report reconciles owners'
  statements to the ledger.
- 🧑‍🤝‍🧑 **Portals as configuration.** Tenants and owners see only what carries their address.
- 🔓 **Open and owned.** AGPL, self-hosted, no per-unit price.
- 🧩 **Extendable on the Ferrum builder.**

### 🚧 Where the leaders are still ahead

- 🧲 **The leasing funnel.** Listings, applications, screening, renewals and e-signed leases.
- 💳 **Collection.** Online rent, autopay, arrears ageing and dunning.
- 🔧 **Work orders and inspections** with vendors, quotes, photos and preventive schedules.
- 🏬 **Commercial leasing.** Percentage rent, options and critical dates.
- 📱 **Mobile apps** and bulk tenant communication.

> [!NOTE]
> Leader columns are from public product information as of November 2026 and may change. Product names belong to
> their owners.

## 📍 Roadmap

### ✅ Recently shipped

- [x] **0.8.2** Tuned design system: better menu icons, lighter first page, cleaner print.
- [x] **0.8.1** The redesigned interface with shaped loading and told-apart states.
- [x] **0.8.0** Owners' expenses from ERP's approved bills; service charges by floor area, equal shares or weighted area, with spending
  read from the bills; the Owner Reconciliation report.
- [x] **0.7.12** Names of records and fields in Arabic and Hindi.
- [x] **0.7.10** The assistant knows the records.
- [x] **0.7.0** Owners' statements: rent collected less the management fee and expenses, billed to the company as the owner's share.
- [x] **0.6.0** Service charge budgets, invoiced with the rent and reconciled once the year is over.

### 🔜 Next up

**Collection**
- [ ] Pay rent online with a payment gateway; autopay and partial payments.
- [ ] A tenant ledger with arrears ageing and dunning notices.
- [ ] Reports: rent roll, vacancy, lease expiry, delinquency and deposits held.

**Leasing**
- [ ] Rental applications, screening and approval into a lease.
- [ ] Lease renewal offers with new terms, and signed leases kept on file.
- [ ] Co-tenants, occupants and guarantors on a lease.
- [ ] Vacancy listings and showings.

**Upkeep**
- [ ] Work orders with vendor assignment, quotes and approval limits; preventive schedules.
- [ ] Inspections with photos, templates by unit type and a signature; move-in against move-out.
- [ ] Deposit deductions itemised from the move-out inspection.

### 🔭 Later

**Owners and money**
- [ ] Trust accounting: owners' funds apart, reserves, more fee types (leasing fees, flat, minimum, per unit) and annual owner statements.
- [ ] One-off and variable charges: meter readings, move-in fees, damage.

**Commercial**
- [ ] Service charge schedules for costs some units do not share (such as a lift), with the landlord bearing vacant units' shares.
- [ ] Percentage rent, options and critical dates.

**Reach**
- [ ] Bulk notices by building, SMS and notice of entry.
- [ ] Richer unit data: bedrooms, bathrooms, amenities, photos and market rent.
- [ ] Tax on rent through a country add-on.
- [ ] Mobile use for inspectors.
- [ ] Costs approved after a statement was billed carried to the next one.

## 🚀 Install

Property is a native Ferrum app: the Ferrum server includes it (`runtime: native`) and a workspace turns it on from **App Manager**. It
needs platform `>=0.3.2, <0.4`. Installing it installs **ERP**, which holds customers, items, invoices and the ledger.

After installing:

1. Give people the roles **Property Manager** and **Facilities**.
2. Open **Property Settings** and set the **company** and the **rent item**. Optionally set a late-fee item and its rules, the deposit
   accounts, the service charge item and the owner payout item.
3. Add **properties** (with owner and manager emails, an owner as a supplier, a management fee and cost centres) and their **units**.
4. Make a **lease** for a unit, add charges and reviews, and **Activate** it. Upkeep starts invoicing.
5. Make tenants and owners **portal people** in Administration, Users, so they see "Your home" and "Properties you own".

## 🔧 Develop

```bash
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
FERRUM_PROPERTY_TEST_DATABASE_URL=postgres://ferrum:ferrum@localhost:5432/ferrum_property_test cargo test
```

The database must exist, be owned by a role that is not a superuser (superusers bypass row-level security) and be reset before the
run. `tests/property_it.rs` walks a year at a block of flats: leases and the unit's days; rent and parking invoiced each month, once;
a quarterly lease cut at its end and a lease ended early; reviews, notice, a late fee and a deposit; days given back by credit note or a
redrawn draft; urgent requests holding a unit; who may do what; the portals; and an owner's statement billed and seen on the portal.
Rust dependencies come from the Ferrum platform and Ferrum ERP repositories on `main`.

The interface lives in `ui/` and is built with Ferrum-Frontend. Check the Frontend out beside the repository as `../ferrum/web`,
install its packages, then:

```bash
cd ui
npm run build       # builds into ui/dist, which the server embeds
npm run check       # interface lint
npm run typecheck
npm test
```

CI (`.github/workflows/ci.yml`) runs Rust on PostgreSQL 17 and the interface checks.

## 📂 Project layout

```text
app.yaml                    manifest: id, roles, shell entries, portal pages, assistant notes
packages/property/          entities, roles and personal-data rules as YAML
src/leases.rs               leases, rent periods, reviews, indexation, notice, late fees, deposits
src/service.rs              service charge budgets and reconciliation
src/statements.rs           owners' statements
src/requests.rs             repair requests and unit status
src/reports.rs              Owner Reconciliation
src/site.rs                 the tenant and owner portal pages
ui/                         the home page and portal pages; words in Arabic and Hindi
tests/property_it.rs        a year at a block of flats on PostgreSQL
docs/PROVENANCE.md          where the design comes from
```

## 📜 Changelog

Every release is described in [CHANGELOG.md](CHANGELOG.md).

## 🤝 Contributing

Issues and pull requests are welcome at [github.com/CBFerrum/ferrum-app-property](https://github.com/CBFerrum/ferrum-app-property/issues).
Keep the checks above green. The design and code of this app are its own; see [docs/PROVENANCE.md](docs/PROVENANCE.md).

## 📄 License

AGPL-3.0-only. See [LICENSE](LICENSE).

<div align="center">

<sub>Part of <a href="https://github.com/CBFerrum/Ferrum">Ferrum</a>, the open business platform · Made with 🦀 Rust and 🐘 PostgreSQL</sub>

</div>
