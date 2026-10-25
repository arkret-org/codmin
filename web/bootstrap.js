import init from "/pkg/codmin.js";
try { await init(); } catch (error) {
  console.error(error);
  const root = document.getElementById("app");
  const message = document.createElement("p");
  message.textContent = "Arkret Admin could not start. Reload the page or contact your administrator.";
  root.replaceChildren(message);
}
