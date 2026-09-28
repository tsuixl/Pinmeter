import { expect } from "@playwright/test";

export async function waitForSettingsSave(page) {
  await expect(
    page.getByRole("button", { name: "恢复默认", exact: true }),
  ).toBeEnabled();
  await expect(
    page.getByRole("alert").filter({ hasText: "保存失败" }),
  ).toHaveCount(0);
}
