import { Globe } from "lucide-react";
import { useState } from "react";
const files = import.meta.glob<string>("./assets/brands/*.ico", {
  eager: true,
  query: "?url",
  import: "default",
});
export function BrandIcon({ id }: { id: string }) {
  const [failed, setFailed] = useState(false);
  const src = files[`./assets/brands/${id}.ico`];
  return (
    <span className="ip-brand" aria-hidden="true">
      {src && !failed ? (
        <img src={src} alt="" onError={() => setFailed(true)} />
      ) : (
        <Globe size={20} />
      )}
    </span>
  );
}
