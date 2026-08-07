+++
type = "presentation"
fullscreen = true
status = "published"
date = "2016-01-28"

slug = "precisionag"
title = "Precision Agriculture"
foot = ""
categories = ["AGRO"]
series = ["POM"]
part = "1"
tags = ["agriculture", "templte", "precision"]

[style]
    accent = "#f72c4a"
    bg = "#d8d9db"
    fg = "#080a0f"

outputs = ["Reveal"]
[reveal_hugo]
    highlight_theme = "arta"
    custom_theme = "cosrel.scss"
    custom_theme_compile = true
+++

{{< slide transition="zoom" transition-speed="fast" class="side-by-side" >}}

### **Sensors, Robotics and Artificial Intelligence in Agrculture**
*future farmers and plant hackers*



---
This presentation uses a custom Reveal.js theme written in SCSS that is compiled by Hugo, using [Hugo pipes](https://gohugo.io/hugo-pipes/).

---
{{< slide background-color="#30ddc3">}}

Hugo pipes compiles the source code of the theme, located in `assets/custom-theme.scss`, into CSS.

---
{{% section %}}

Hugo pipes compiles the source code of the theme, located in `assets/custom-theme.scss`, into CSS.

---
{{< slide background-color="#30b2dd">}}

Hugo pipes compiles the source code of the theme, located in `assets/custom-theme.scss`, into CSS.

---

Hugo pipes compiles the source code of the theme, located in `assets/custom-theme.scss`, into CSS.
{{% /section %}}

---

{{< slide background-color="#857cea">}}
No separate build process is required, which means you can iterate on your theme and your content at the same time.

---

To set a custom theme and use Hugo to compile it:

```toml
[reveal_hugo]
custom_theme = "custom-theme.scss"
custom_theme_compile = true
```

`custom-theme.scss` must live in the `assets` folder.

---

To pass compilation options, use the `custom_theme_options` param:

```toml
[reveal_hugo.custom_theme_options]
targetPath = "css/custom-theme.css"
enableSourceMap = true
```

See all the [available options](https://gohugo.io/hugo-pipes/scss-sass/#options).

---

{{< slide >}}

Note: to use a custom theme that doesn't need compilation, put it in the `static` directory and do not set the `custom_theme_compile` parameter. It will be served to the browser directly.

---

Write a custom Reveal.js theme in four steps:

```text
1. Import `mixins` and `settings` from the templates directory
2. Override variables and functions for colors, fonts and sizes
3. Import `theme` from the templates directory
4. Add any additional selectors to override the built CSS
```

---

See the [Reveal.js theme docs](https://github.com/hakimel/reveal.js/blob/master/css/theme/README.md) to learn what overrides are possible.

---

See `assets/custom-theme.scss` or any file in `assets/reveal-js/css/theme/source` for an example.
