import { describe, expect, it } from "vitest";
import { byBuilding, endingSoon } from "./Home";

describe("the units page", () => {
  it("groups units by building", () => {
    const m = byBuilding([
      { id: "u1", building: "b1" },
      { id: "u2", building: "b2" },
      { id: "u3", building: "b1" },
    ]);
    expect(m.get("b1")?.map((u) => u.id)).toEqual(["u1", "u3"]);
    expect(m.get("b2")?.length).toBe(1);
  });

  it("lists active leases ending within the window, soonest first", () => {
    const l = endingSoon(
      [
        { id: "late", status: "Active", ends_on: "2027-03-01" },
        { id: "b", status: "Active", ends_on: "2026-12-20" },
        { id: "a", status: "Active", ends_on: "2026-11-30" },
        { id: "ended", status: "Ended", ends_on: "2026-11-15" },
        { id: "past", status: "Active", ends_on: "2026-10-01" },
        { id: "noticed", status: "Active", ends_on: "2027-07-31", leaving_on: "2026-12-01", notice_by: "Lessee" },
      ],
      "2026-11-04",
    );
    expect(l.map((x) => x.id)).toEqual(["a", "noticed", "b"]);
  });
});

describe("the portal's home", () => {
  it("offers repairs only in units rented now", async () => {
    const { rentedNow } = await import("./Portal");
    const l = rentedNow([
      { id: "a", status: "Active", unit: "u1", invoices: [] },
      { id: "e", status: "Ended", unit: "u2", invoices: [] },
      { id: "t", status: "Terminated", unit: "u3", invoices: [] },
    ]);
    expect(l.map((x) => x.unit)).toEqual(["u1"]);
  });
});
