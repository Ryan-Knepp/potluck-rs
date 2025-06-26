(function () {
  const html = document.documentElement;
  const toggleBtn = document.getElementById("theme-toggle");
  const icon = document.getElementById("theme-toggle-icon");

  function setTheme(theme) {
    if (theme === "dark") {
      html.classList.add("dark");
      icon.textContent = "☀️";
    } else {
      html.classList.remove("dark");
      icon.textContent = "🌙";
    }
    localStorage.setItem("theme", theme);
  }

  // On load, set theme from localStorage or system preference
  const savedTheme = localStorage.getItem("theme");
  if (savedTheme) {
    setTheme(savedTheme);
  } else if (window.matchMedia("(prefers-color-scheme: dark)").matches) {
    setTheme("dark");
  } else {
    setTheme("light");
  }

  toggleBtn.addEventListener("click", () => {
    const isDark = html.classList.contains("dark");
    setTheme(isDark ? "light" : "dark");
  });
})();
