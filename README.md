# Stembed — Steno, embedded.

Project to develop a fully embeddable Stenography engine that can run virtually anywhere with just a few kilobytes of memory and very little computing power.

For more details, consult the [`docs/`](/docs) directory and make sure to read the [Goals](Goals.md)! This project is in a pre-alpha state and it is not recommended to use it unless you really know what you are getting yourself into.

## FAQ

- *Does this compete with Plover?* In a way. See below.
- *Are you happy about this?* No.
- *Is it necessary?* Unfortunately, yes.
- *Would you be open to collaborating with Plover?* Absolutely! Although at this time I do not see a way of integrating this project with Plover which does not involve scrapping their current codebase. See below for more info.

### What about Plover?

Let me start off by stating that Plover is a fabulous piece of software. Without it and the work of each and every collaborator I would never have gotten into Stenography. It introduced countless people into this hobby and some even made it their job which is fantastic in its own right! The project has been around for a comparatively long while — originating from an idea by Mirabai Knight, it organically grew over the years and amassed both more features and an ever growing community.

During its infancy, a number of technical decisions have been made which includes the choice of a programming language: Python. While this may have been a sound choice at the time (Python is great for fast prototyping and "dynamic" coding), it severely limits the possibilities nowadays. While Python runs on every major operating system, it is virtually impossible to execute it on mobile phones, embedded devices, or even the web (this is the reason why you see multiple projects like e.g. Dotterel, crides/steno, or TypeyType "reinventing" the wheel when it comes to converting steno strokes into text). Additionally, the code base has been written with desktop computers in mind where an abundance of computing power and memory is available. For this reason, I deem it a necessary step in the evolution of hobby stenography to have a open Steno engine written in a language that is portable across *all* platforms and architected in a way so that it can run with very little resources available. That is the rationale and vision behind this project.

Now you know why I deemed it necessary to diverge from the existing Plover codebase instead of contributing to it. All that said, I am always open for discussion regarding any of this and would love to see the community stay together rather than diverge – maybe someday it would even be thinkable to integrate this project into the "Plover" universe by potentially replacing the old Python based engine (though that certainly requires the addition of a lot of features into this engine before such a thing is even remotely feasible).
