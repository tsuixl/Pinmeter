import type { HardwareInfoDto } from "../contracts/monitor";

export function demoHardware(): HardwareInfoDto {
  return {
    status: "ready",
    detail: "演示数据",
    manufacturer: "ASUS（演示）",
    model: null,
    system: "Microsoft Windows 11 专业版",
    system_detail: "64 位 · 10.0.26200",
    boot_at_ms: Date.now() - 7 * 86_400_000 - 6 * 3_600_000,
    sections: [
      {
        id: "board",
        error: null,
        items: [
          {
            name: "ROG CROSSHAIR X870E HERO",
            details: ["ASUSTeK COMPUTER INC.", "BIOS 2202"],
          },
        ],
      },
      {
        id: "cpu",
        error: null,
        items: [
          { name: "AMD Ryzen 9 9950X3D", details: ["16 核", "32 逻辑处理器"] },
        ],
      },
      {
        id: "memory",
        error: null,
        items: [
          { name: "已安装 48 GiB", details: ["2 / 4 插槽已使用"] },
          { name: "Asgard DDR5", details: ["24 GiB × 2", "配置速率 5600"] },
        ],
      },
      {
        id: "gpu",
        error: null,
        items: [
          { name: "NVIDIA GeForce RTX 5070 Ti（演示）", details: [] },
          { name: "AMD Radeon 核显（演示）", details: ["集成显卡"] },
        ],
      },
      {
        id: "display",
        error: null,
        items: [
          {
            name: "XG27UCG",
            details: ["AUS", "约 27.2 英寸", "首选 3840 × 2160"],
          },
          { name: "S2719DGF", details: ["DEL", "首选 2560 × 1440"] },
        ],
      },
      {
        id: "disk",
        error: null,
        items: [
          {
            name: "ZHITAI TiPlus7100 2TB",
            details: ["1907.7 GiB", "SSD", "NVMe"],
          },
          { name: "TOSHIBA HDWD130", details: ["2794.5 GiB", "HDD", "SATA"] },
        ],
      },
      {
        id: "audio",
        error: null,
        items: [
          { name: "Realtek USB Audio", details: [] },
          { name: "NVIDIA High Definition Audio", details: [] },
        ],
      },
      {
        id: "network",
        error: null,
        items: [
          { name: "Realtek PCIe 5GbE Family Controller", details: ["以太网"] },
          {
            name: "Qualcomm FastConnect 7800 Wi-Fi 7 High Band Simultaneous (HBS) Network Adapter",
            details: ["WLAN"],
          },
        ],
      },
    ],
  };
}
