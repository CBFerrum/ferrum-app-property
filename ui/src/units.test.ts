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

describe("the owners' page", () => {
  it("names a period's month, and leaves anything else as it came", async () => {
    const { monthName } = await import("./Portal");
    expect(monthName("2026-08", "en-GB")).toBe("August 2026");
    expect(monthName("2026-01", "en-GB")).toBe("January 2026");
    expect(monthName("2026-13", "en-GB")).toBe("2026-13");
    expect(monthName("soon")).toBe("soon");
  });

  it("writes amounts as numbers, keeping their decimals", async () => {
    const { amount } = await import("./Portal");
    expect(amount("20900", "en-GB")).toBe("20,900");
    expect(amount("44505.49", "en-GB")).toBe("44,505.49");
    expect(amount("0.125", "en-GB")).toBe("0.125");
    expect([amount(null), amount(""), amount("n/a")]).toEqual(["", "", "n/a"]);
  });
});

describe("owners' statements", () => {
  it("say where the payout stands", async () => {
    const { payoutState } = await import("./Portal");
    const s = { number: "OS-1", period_from: "2026-08-01", period_to: "2026-10-31", collected: "12000", management_fee: "1200", expenses: "1300", payout: "9500" };
    expect(payoutState(s)).toBe("Waiting for accounts");
    expect(payoutState({ ...s, still_to_pay: "9500" })).toBe("9500 to pay");
    expect(payoutState({ ...s, still_to_pay: "0" })).toBe("Paid");
  });
});
