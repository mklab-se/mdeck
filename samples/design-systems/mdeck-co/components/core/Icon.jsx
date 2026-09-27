import React from "react";

/**
 * MDeck Co Icon: thin wrapper over Lucide (loaded from CDN).
 * Lucide's 1.5px stroke matches the brand's precise, quiet line.
 * Requires the Lucide UMD script to be present on the page:
 *   <script src="https://unpkg.com/lucide@latest"></script>
 */
export function Icon({ name, size = 18, strokeWidth = 1.5, color = "currentColor", style = {}, ...rest }) {
  const ref = React.useRef(null);

  React.useEffect(() => {
    if (window.lucide && ref.current) {
      ref.current.innerHTML = "";
      const el = document.createElement("i");
      el.setAttribute("data-lucide", name);
      ref.current.appendChild(el);
      try {
        window.lucide.createIcons({
          attrs: { width: size, height: size, "stroke-width": strokeWidth },
          nameAttr: "data-lucide",
        });
      } catch (e) { /* noop */ }
    }
  }, [name, size, strokeWidth]);

  return (
    <span
      ref={ref}
      aria-hidden="true"
      style={{
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        width: size,
        height: size,
        color,
        flexShrink: 0,
        ...style,
      }}
      {...rest}
    />
  );
}
