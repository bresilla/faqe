+++
type = "post"
status = "published"
date = "2020-08-07"
readingtime = 5

slug = "nix-pckgs"
title = "Using nix-shell as package manager for any language"
thumbnail = "thumbnail.png"
foot = "There are more dead people than living, and their numbers are increasing - Eugene Ionesco"
categories = ["DEV"]
series = ["NIX-SHELL"]
part="1"
tags = ["direnv", "nix-shell", "package", "language" ]


description = "use NIX tools such as: NixOS itself, nix-shell and nix-pkgs in a development environment."
punchline = "Getting a working development environment is often far from an easy task. Actually, setting up projects can take days before getting everything working together on a development machine."
tldr = "THINGS ARE ABOUT TO GET MESSY"

credits = [
    "https://ocharles.org.uk/blog/posts/2014-02-04-how-i-develop-with-nixos.html",
    "https://en.wikipedia.org/wiki/Dependency_hell",
    "https://medium.com/swlh/dependency-management-a6cce61660d",
    "https://devopedia.org/dependency-manager",
    "https://medium.com/omarelgabrys-blog/software-engineering-software-process-and-software-process-models-part-2-4a9d06213fdc",
    "https://medium.com/@simon.maxen/avoiding-dependency-hell-4121d2716918",
]

[style]
    accent = "#5fd7ff"
    theme = "dark"
    width="60%"

+++


# Software development

Software design and development is a very complicated process. It is a set of related activities that let to the production of an application or software. Those activites may involve the development of the software from scratch or a more tidious process of modyfiying an existing system.

Software might be the first kind of knowledge that can be composed. It is hard to develop, but it is often easy to use, and is not even that hard to combine. And the latter is what brings the real magic. It is the closest thing to “building on the shoulders of giants” in the realm of really building stuff.

## Dependencies
A good practice in software design is to build software from smaller, single-purpose modules that exposes well-defined interfaces. This is also in the spirit of software reuse. With the widespread adoption of open source software, often applications or modules depend on other applications and modules. Thus, to build an application we need to bring in all parts on which it depends, including language libraries and remote third-party modules.

so:


{{< block type="fill" >}}
Every applicaiton on a computer implicitly depends on a whole bunch of other things on the same computer.
{{< /block >}}

thus:

- All software exists in a graph of dependencies.
- Most of the time, this graph is implicit.

## Dependency hell
“Dependency hell” is a term for the frustration that arises from problems with transitive (indirect) dependencies.

Sharing commonly-used code in the form of libraries is one of the key aspects of software development. Ease of sharing code libraries combined with easy discovery build ecosystems is one of the greates value for an ecosystem to be successfully be that a programming language or a framework. There are many good examples like: NodeJS’s npm, Rust’s crates, Python’s PyPI among many of them.

The big problem is the fact that small even the smallest libraries pull in additional depedencies on which they rely in order to avoid too much code duplication And as those packages or modules or libraries increases in adoption, they get bundled in more and more packages. While this is a nice thing to have, the flip side is that we see multiple levels of dependencies build up as these packages are in turn used by others and those by others.

{{< block type="fill" >}}
If not carefull with dependency management, one may end up with a spaghetti of them.
{{< /block >}}

## The trap

Lets just describe a very simple dependency hell [credits goes to [this guy](https://medium.com/@simon.maxen/avoiding-dependency-hell-4121d2716918)]:

### The App
We have a deployable application `A`, that depends on `j-1.0` and `k-1.0` which in turn depends on `d-1.0` and there is only one version of everything and all projects live in their own version control.

### The Trap
Now we have an a new version of `j-2.0` added to the system that depends on a new version of `d-2.0`. As application `A` does not use the new version of `j-2.0` but still uses `j-1.0`, everything is still working fine.

### The Trouble
The application `A` developer now wants to use the latest version of `j-2.0` as it has some great performance improvements. However we now have a problem as `j-2.0` depends on `d-2.0` and `k-1.0` depends on `d-1.0` but only one version of `d` can be used.

## Froms
Dependeny hell has many forms. The example above is what is calld a diamond dependency. According to Wikipedia there are 6 kind of hell (sorry [Dante](https://en.wikipedia.org/wiki/Inferno_(Dante))):

- Many dependencies
- Long chains of dependencies
- Conflicting dependencies
- Circular dependencies
- Package manager dependencies
- Diamond dependency

## So what now??
But it's not trivial to ensure that we have all necessary dependencies, particularly when dependencies themselves depend on others. This is why we need a Dependency Manager.

{{< block type="fill" >}}
Dependency management is a technique for declaring, resolving and using dependencies required by the project in an automated fashion.
{{< /block >}}

Dependency managers bring together all necessary project dependencies before passing them to the compiler or interpreter. In this sense, their work is somewhat a preprocessing phase before the actual build happens.

# Enter NIX

It’s difficult to categorize Nix. On the website, it is presented as a “purely functional package manager”. However, it could also be thought of as a programming language, a configuration tool, and a build tool. In a nutshell, Nix expressions are programs written in a purely functional programming language and can return a single derivation or a set of derivations.

This difficulty to categorize comes due to a few different projects sharing similar names. So, lets just touch them breifly before we dig in:

- Nix: A purely functional package manager with the associated programming language called Nix expression.
- NixOS: an operating system that uses the power of Nix and Nix expressions to provide system configuration management.
- Nix-pkgs: the common repository of software that defines how to install rust, go, vim, pip, etc.
- Nix-shell: starts an interactive shell based on a Nix expression that either builds or fetches cached builds of dependencies and adds them to the Nix store.
- Nix-store: is the path where immutable build packages are stored. Since every build/version has its own path, packages are atomic.

The key take-away of NIS it's deterministic and reproducable builds. Meaning that, given some Nix-expression, it doesn’t matter on what computer or what time you attempt to build it, it will produce the same result. Nix has gone to great lengths to ensure reproducible builds, where not just the project’s direct dependencies are locked, but practically every external dependency up to and including the global system. It formalizes and encapsulates these tools in a way that they are locked to a given version.


# Enter NIX
