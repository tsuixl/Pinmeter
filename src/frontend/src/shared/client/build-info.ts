import manifest from "../../../package.json";
declare const __BUILD_REVISION__: string;
declare const __BUILD_TIME__: string;

export const buildInfo = {
  version: manifest.version,
  revision:
    typeof __BUILD_REVISION__ === "string" ? __BUILD_REVISION__ : "开发环境",
  time: typeof __BUILD_TIME__ === "string" ? __BUILD_TIME__ : "",
};
