// Development fixture only; public example addresses, never a production fallback.
import type { IpStateDto } from "../contracts/monitor";
export function demoIp(): IpStateDto {
  const address = "1.1.1.1";
  return {
    revision: "1",
    generation: "1",
    running: false,
    retry_after_ms: 0,
    checks: [
      {
        id: "connectivity",
        ids: [
          "bytedance",
          "wechat",
          "taobao",
          "youtube",
          "cloudflare",
          "github",
        ],
      },
      {
        id: "ai",
        ids: [
          "deepseek",
          "qwen",
          "kimi",
          "chatgpt",
          "claude",
          "perplexity",
          "grok",
          "gemini",
        ],
      },
      {
        id: "services",
        ids: [
          "openai",
          "cloudflare",
          "supabase",
          "claude",
          "perplexity",
          "cursor",
          "github",
          "vercel",
        ],
      },
    ].map(({ id, ids }) => ({
      id,
      running: false,
      retry_after_ms: 0,
      rows: ids.map((name, index) => ({
        id: name,
        status: "ready",
        valid_at_ms: Date.now(),
        error: null,
        samples:
          id === "connectivity"
            ? [
                63 + index * 20,
                70 + index * 20,
                65 + index * 20,
                68 + index * 20,
                66 + index * 20,
                69 + index * 20,
                72 + index * 20,
                67 + index * 20,
              ]
            : id === "ai"
              ? [180 + index * 95]
              : [],
        http_status: id === "services" ? null : 200,
        latency_ms:
          id === "services"
            ? null
            : id === "ai"
              ? 180 + index * 95
              : 68 + index * 20,
        service_state:
          id === "services" ? (index === 0 ? "degraded" : "operational") : null,
        source: id === "services" ? "演示：官方状态样例" : null,
        updated_at: null,
        incidents:
          id === "services" && index === 0
            ? [
                {
                  name: "演示事件：部分服务延迟",
                  state: "investigating",
                  updated_at: null,
                },
              ]
            : [],
        components:
          id === "services"
            ? [{ name: "演示组件", state: "operational", updated_at: null }]
            : [],
      })),
    })),
    exits: [
      {
        id: "ipv4",
        status: "ready",
        address,
        source: "演示目标",
        route: "演示路由，不代表本机出口",
        valid_at_ms: Date.now(),
        error: null,
      },
      {
        id: "ipv6",
        status: "ready",
        address: "2606:4700:4700::1111",
        source: "演示目标",
        route: "演示路由",
        valid_at_ms: Date.now(),
        error: null,
      },
      {
        id: "domestic",
        status: "failed",
        address: null,
        source: "",
        route: "",
        valid_at_ms: null,
        error: "演示：目标没有返回可读取的出口",
      },
    ],
    profiles: [
      {
        address,
        status: "ready",
        valid_at_ms: Date.now(),
        error: null,
        data: {
          address,
          source: "演示数据 · Net.Coffee 字段样例",
          country: "澳大利亚",
          region: "Queensland",
          city: "South Brisbane",
          isp: "Cloudflare, Inc.",
          asn: 13335,
          score: 75,
          facts: [
            { key: "cidr", value: "1.1.1.0/24" },
            { key: "reverse_dns", value: "one.one.one.one" },
            { key: "organization", value: "Cloudflare, Inc." },
            { key: "registered_country", value: "澳大利亚" },
            { key: "rpki", value: "valid" },
          ],
          flags: [
            { key: "residential", value: false },
            { key: "datacenter", value: true },
            { key: "vpn", value: null },
            { key: "proxy", value: false },
            { key: "abuser", value: null },
          ],
          locations: [
            {
              source: "Net.Coffee / 演示 A",
              country: "澳大利亚",
              region: "Queensland",
              city: "South Brisbane",
            },
            {
              source: "Net.Coffee / 演示 B",
              country: "澳大利亚",
              region: "New South Wales",
              city: "Sydney",
            },
          ],
          records: [
            {
              kind: "related_domains",
              value: "one.one.one.one",
              detail: "reverse DNS",
              at: null,
            },
          ],
          truncated: false,
        },
      },
    ],
  };
}
