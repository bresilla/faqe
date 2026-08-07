+++
type = "post"
status = "published"
date = "2020-06-24"
readingtime = 2

slug = "direnv-anaconda"
title = "Using *direnv* with Anaconda"
thumbnail = "thumbnail.png"
foot = "Artificial Intelligence is no match for natural stupidity - AI itself"
description = "How to use direnv to manage, arranga, list... projects"
categories = ["DEV"]
series = ["DIRENV"]
part="1"
tags = ["direnv", "anaconda", "python" ]


[style]
    accent = "#ee82ee"
    theme = "dark"
    width="60%"
+++

## Introduction
This tutorial will show you how to create a simple theme in Hugo. I assume that you are familiar with HTML, the bash command line, and that you are comfortable using Markdown to format content. I'll explain how Hugo uses templates and how you can organize your templates to create a theme. I won't cover using CSS to style your theme.


We'll start with creating a new site with a very basic template. Then we'll add in a few pages and posts. With small variations on that, you will be able to create many different types of web sites.

In this tutorial, commands that you enter will start with the "$" prompt. The output will follow. Lines that start with "#" are comments that I've added to explain a point. When I show updates to a file, the ":wq" on the last line means to save the file.

Here's an example:

```
## this is a comment
$ echo this is a command
this is a command

## edit the file
$vi foo.md
+++
date = "2014-09-28"
title = "creating a new theme"
+++

bah and humbug
:wq

## show it
$ cat foo.md
+++
date = "2014-09-28"
title = "creating a new theme"
+++

bah and humbug
$
```


## Some Definitions

There are a few concepts that you need to understand before creating a theme.

### Skins

Skins are the files responsible for the look and feel of your site. It’s the CSS that controls colors and fonts, it’s the Javascript that determines actions and reactions. It’s also the rules that Hugo uses to transform your content into the HTML that the site will serve to visitors.
