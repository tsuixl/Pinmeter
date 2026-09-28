export const names: Record<string, string> = {
  bytedance: "字节跳动",
  wechat: "微信",
  taobao: "淘宝",
  youtube: "YouTube",
  cloudflare: "Cloudflare",
  github: "GitHub",
  deepseek: "DeepSeek",
  qwen: "Qwen",
  kimi: "Kimi",
  chatgpt: "ChatGPT",
  claude: "Claude",
  perplexity: "Perplexity",
  grok: "Grok",
  gemini: "Gemini",
  openai: "OpenAI",
  supabase: "Supabase",
  cursor: "Cursor",
  vercel: "Vercel",
};
export const serviceLabels: Record<string, string> = {
  operational: "正常运行",
  degraded: "服务降级",
  degraded_performance: "性能下降",
  partial_outage: "部分故障",
  major_outage: "严重故障",
  maintenance: "维护中",
  under_maintenance: "维护中",
  investigating: "调查中",
  identified: "已定位",
  monitoring: "监测恢复",
  resolved: "已恢复",
  unknown: "待确认",
};
export const formatTime = (value: number | null | undefined) =>
  value == null ? "尚未获取数据" : new Date(value).toLocaleString();
export const scoreTone = (score: number | null | undefined) =>
  score == null
    ? "neutral"
    : score >= 75
      ? "success"
      : score >= 45
        ? "warning"
        : "danger";
