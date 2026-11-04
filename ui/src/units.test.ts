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
      ],
      "2026-11-04",
    );
    expect(l.map((x) => x.id)).toEqual(["a", "b"]);
  });
});
