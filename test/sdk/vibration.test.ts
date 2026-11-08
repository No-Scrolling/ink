import { expect, test } from "bun:test";
import { vibrate } from "../../packages/vibration/src/index";

test.each([-1, 0.5, NaN, Infinity, Number.MAX_SAFE_INTEGER + 1])(
  "vibration rejects invalid duration %s before calling Android",
  async (ms) => {
    await expect(vibrate(ms)).rejects.toBeInstanceOf(RangeError);
    await expect(vibrate([{ ms, strength: 1 }])).rejects.toBeInstanceOf(RangeError);
  },
);

test.each([-0.1, 1.1, NaN, Infinity])("vibration rejects invalid strength %s", async (strength) => {
  await expect(vibrate(200, { strength })).rejects.toBeInstanceOf(RangeError);
  await expect(vibrate([{ ms: 200, strength }])).rejects.toBeInstanceOf(RangeError);
});

test("vibration rejects empty patterns", async () => {
  await expect(vibrate([])).rejects.toBeInstanceOf(RangeError);
});
