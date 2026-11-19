// Links with an explicit download attribute can be handed off during the
// user's click, which lets the browser open the registered desktop protocol
// even when background download events cannot initiate external applications.
document.addEventListener("click", (event) => {
  if (!(event.target instanceof Element)) return;

  const link = event.target.closest("a[download][href]");
  if (!link || event.defaultPrevented || event.button !== 0 || event.ctrlKey || event.metaKey || event.shiftKey || event.altKey) {
    return;
  }

  const target = new URL(link.href, document.baseURI);
  if (target.protocol !== "http:" && target.protocol !== "https:") return;

  event.preventDefault();
  event.stopImmediatePropagation();
  window.location.href = "iranidm://download?url=" + encodeURIComponent(target.href);
}, true);
