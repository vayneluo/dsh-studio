document.documentElement.classList.add("has-js");

const revealElements = document.querySelectorAll("[data-reveal]");

if (!("IntersectionObserver" in window)) {
  revealElements.forEach((element) => element.classList.add("is-visible"));
} else {
  const observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (!entry.isIntersecting) continue;
        entry.target.classList.add("is-visible");
        observer.unobserve(entry.target);
      }
    },
    { rootMargin: "0px 0px -10%", threshold: 0.08 },
  );

  revealElements.forEach((element) => observer.observe(element));
}
