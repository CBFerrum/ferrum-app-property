-- Property: the database's own guard against letting a unit twice. A lease past its draft holds its
-- unit from its first day to its last, both included: its end, or sooner when it was ended early or
-- notice was given (the least of the three, as leases.rs's last_day). No two leases of a unit hold
-- the same day. Activating a lease refuses an overlap first, naming the other lease; this holds
-- when two are activated at once (src/lib.rs registers its words).
alter table e_lease add constraint property_unit_let_once
  exclude using gist (tenant_id with =, unit with =, daterange(starts_on, least(ends_on, terminated_on, leaving_on), '[]') with &&)
  where (status <> 'Draft');
