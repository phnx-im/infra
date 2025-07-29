// #let default-color = black.darken(40%)
// #let header-color = default-color.lighten(75%)
// #let body-color = default-color.lighten(85%)
#let default-color = rgb("#404040")
#let header-color = rgb("#404040")
#let body-color = rgb("#ff0000")

#let layouts = (
  "small": ("height": 9cm, "space": 1.4cm),
  "medium": ("height": 10.5cm, "space": 1.6cm),
  "large": ("height": 12cm, "space": 1.8cm),
)

#let slides(
  content,
  title: none,
  subtitle: none,
  date: none,
  author: (),
  layout: "large",
  ratio: 16/9,
  title-color: none,
  lang: ("en", "us"),
) = {
  if (type(author) != array) {
    author = (author,)
  }

  // Parsing
  if layout not in layouts {
      panic("Unknown layout " + layout)
  }
  let (height, space) = layouts.at(layout)
  let width = ratio * height

  // Colors
  if title-color == none {
      title-color = default-color
  }

  // Setup
  set document(
    title: title,
    author: author,
  )

  set text(
    font: "Inter",
    lang: lang.at(0),
    region: lang.at(1),
  )

  set par(
    // justify: true,
    leading: 1.0em,
    spacing: 1em,
  )

  set block(
    below: 2em,
  )


  set page(
    fill: rgb("#ffffff"),
    width: width,
    height: height,
    margin: (x: 0.5 * space, top: 1.2 * space, bottom: 0.6 * space),
    header: context {
      let page = here().page()
      let headings = query(selector(heading.where(level: 2)))
      let heading = headings.rev().find(x => x.location().page() <= page)
      if heading != none {
        set align(top)
        // Normal slide titles
        set text(
          1.5em,
          weight: "bold",
          font: "Inter",
          number-type: "lining",
          number-width: "tabular",
          fill: title-color
        )
        v(space / 2)
        block(heading.body +
          if not heading.location().page() == page [
            #{numbering("(1)", page - heading.location().page() + 1)}
          ]
        )
      }
    },
    header-ascent: 0%,
    footer: {
      set text(8pt,
        font: "Inter",
        weight: "light")
      title
      " - "
      author.join(",")
      h(1fr)
      context counter(page).display("1/1", both: true)
    },
    footer-descent: 10pt,
  )

  set outline(
    target: heading.where(level: 1),
    title: none,
  )

  set bibliography(
    title: none,
    style: "turabian-fullnote-8"
  )

  // Rules
  show heading.where(level: 1): x => {
    set page(
      header: context {
        set align(top)
        // Outline heading
        set text(
          2.5em,
          weight: "bold",
          font: "Inter",
          number-type: "lining",
          number-width: "tabular",
          fill: title-color
          )
        v(space / 2)
        if lang.at(0) == "de" {
          "Übersicht"
        } else {
          "Outline"
        }
      },
      footer: {
        set text(8pt,
          font: "Inter",
          weight: "light")
        title
        " - "
        author.join(",")
        h(1fr)
        context counter(page).display("1/1", both: true)
      },
      footer-descent: 10pt,
    )

    show outline.entry: it => {
      if it.element == x {
        text(1em, font: "Inter", it)
      } else {
        text(1em, font: "Inter", fill: gray, it)
      }
    }
    // Outline entries
    set text(
      0.7em,
      weight: "bold",
      font: "Inter",
      number-type: "lining",
      number-width: "tabular",
      fill: default-color,
    )

    outline()
    pagebreak()
  }

  show heading.where(level: 2): pagebreak(weak: true)
  show heading: set text(1.1em, fill: title-color)

  // Title
  if (title == none) {
    panic("A title is required")
  } else {

    set page(footer: none)
    set align(center+horizon)
    v(- space / 2)

    // The main title
    block(
      text(
        2.5em,
        weight: "bold",
        font: "Inter",
        number-type: "lining",
        number-width: "tabular",
        fill: title-color,
        title
      ) +
      v(1.4em, weak: true) +
      if subtitle != none { text(1.1em, weight: "bold", subtitle) } +
      if subtitle != none and date != none { text(1.1em)[ \- ] } +
      if date != none {text(1.1em, date.display())} +
      v(1.75em, weak: true) +

      for a in author [
        #text(1.1em, a)\
      ]
    )
  }

  // Content
  content
}

#let frame(content, counter: none, title: none) = {

  let header = none
  if counter == none and title != none {
    header = [*#title.*]
  }
  else if counter != none and title == none {
    header = [*#counter.*]
  }
  else {
    header = [*#counter:* #title.]
  }

  set block(width: 100%, inset: (x: 0.4em, top: 0.35em, bottom: 0.45em))
  show stack: set block(breakable: false)
  show stack: set block(breakable: false, above: 0.8em, below: 0.5em)

  stack(
    block(fill: header-color, radius: (top: 0.2em, bottom: 0cm), header),
    block(fill: body-color, radius: (top: 0cm, bottom: 0.2em), content),
  )
}

#let d = counter("definition")
#let definition(content, title: none) = {
  d.step()
  frame(counter: d.display(x => "Definition " + str(x)), title: title, content)
}

#let t = counter("theorem")
#let theorem(content, title: none) = {
  t.step()
  frame(counter: t.display(x => "Theorem " + str(x)), title: title, content)
}

#let l = counter("lemma")
#let lemma(content, title: none) = {
  l.step()
  frame(counter: l.display(x => "Lemma " + str(x)), title: title, content)
}

#let c = counter("corollary")
#let corollary(content, title: none) = {
  c.step()
  frame(counter: c.display(x => "Corollary " + str(x)), title: title, content)
}

#let a = counter("algorithm")
#let algorithm(content, title: none) = {
  a.step()
  frame(counter: a.display(x => "Algorithm " + str(x)), title: title, content)
}

#show: slides.with(
  title: "Mimi Content Format",
  author: ("Timo Kösters <timo@koesters.xyz>"),
  lang: ("en", "US"),
  //lang: ("de", "DE")
)

= The current message format
== An image and a caption

```
render, id:1, multi, processAll
- inline, id:2, external, type:image/png
- render, id:3, single, type:text/markdown, content:"Look at this!"
```

#image("caption.png", height: 70%)

== A message with an image

```
render, id:1, multi, processAll
- render, id:2, single, type:text/markdown, content:"![](cid:3)\nLook at this!"
- attachment, id:3, external, type:image/png
```

#image("caption.png", height: 70%)


== Do we want internal references?

#table(
  columns: (auto, auto),
  inset: 10pt,
  align: horizon,
  table.header(
    [*No references*], [*Internal references*],
  ),
  [Application has more control over rendering], [Sender has more control over rendering],
  [Easier to parse], [Parsing has a lot of edge cases],
)


== Do we want deep nesting?

```
render, id:1, multi, chooseOne
- english, render, id:2, multi, chooseOne
    - german, render, id:3, single, type:text/markdown, content:"![](cid:3)"
    - french, render, id:4, single, type:text/markdown, content:"![](cid:5)"
- german, render, id:5, multi, chooseOne
    - english, render, id:6, single, type:text/markdown, content:"![](cid:2)"
    - french, render, id:7, single, type:text/markdown, content:"![](cid:8)"
- french, attachment, id:8, external, type:image/png
```


= Timo's Proposals

== markdownChooseOne

- Every message is a markdown message that can reference attachments to display them inline
- Unreferenced attachments are not displayed inline, but can be downloaded

Pros:
- Clear separation between message and attachments
- Simple chooseOne decision

Cons:
- No nested attachments: message -> vcard -> image
- Does not have fallback for clients without markdown support
- Requires understanding of markdown
- Is not strictly using markdown correctly
  - "inline images" are used for any attachments
  - `![](cid:1)` Looks ugly, maybe use link reference `[1]` instead

```
markdownChooseOne:
- english: "Image: ![](cid:1), Video: ![](cid:2)"
- german: "Foto: ![](cid:1), Film: ![](cid:2)"

attachments:
cid:1 inline chooseOne:
    - english: image
    - german: image
cid:2 inline chooseOne
    - english: video
cid:3 attachment chooseOne
    - english: pdf

```

== documentChooseOne

- Add content types
- Add dispositions to attachments

```
documentChooseOne:
- english plain: "Image: [1], Video: [2]"
- english markdown: "Image: ![](cid:1), Video: ![](cid:2)"
- english html: "<p>Image: <img src="cid:1" />, Video: <video src="cid:2"></video></p>"

attachments:
cid:1 inline chooseOne:
    - english: image
    - german: image
cid:2 inline chooseOne
    - english: video
cid:3 attachment chooseOne
    - english: pdf
```


= Questions
== Questions

- Do we want internal references?
- Do we want deep nesting?
- Are the proposals better than the draft?
- Should clients be required to understand markdown?
