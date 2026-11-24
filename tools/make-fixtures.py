# SPDX-License-Identifier: MIT OR Apache-2.0
# Makes the DOCX, ODT and RTF fixtures in tests/fixtures/ from the public-domain texts there.
# Python standard library only; deterministic (fixed zip times, sorted output), so running it
# again gives the same bytes. Usage, from the repository root:
#
#     python3 tools/make-fixtures.py
#
# The files are "written for the test": they hold the novels' words in the simplest valid form of
# each format (paragraphs, Heading 1 for chapter titles, italic from the texts' _underscores_).
# docs/fixtures.md records what each file is.

import os
import re
import zipfile
from xml.sax.saxutils import escape

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tests", "fixtures")
STAMP = (1980, 1, 1, 0, 0, 0)
CHAPTER = re.compile(r"^chapter [IVXLC]+\.?\]?$", re.IGNORECASE)
ITALIC = re.compile(r"_([^_]+)_")


def paragraphs(text):
    """Blocks separated by blank lines; the lines of a block joined by single spaces."""
    out = []
    for block in re.split(r"\n\s*\n", text):
        lines = [l.strip() for l in block.split("\n") if l.strip()]
        if lines:
            out.append(" ".join(lines))
    return out


def runs(par):
    """(text, italic) pieces of a paragraph."""
    pieces, pos = [], 0
    for m in ITALIC.finditer(par):
        if m.start() > pos:
            pieces.append((par[pos:m.start()], False))
        pieces.append((m.group(1), True))
        pos = m.end()
    if pos < len(par):
        pieces.append((par[pos:], False))
    return pieces


def is_heading(par):
    return bool(CHAPTER.match(par))


def write_zip(path, entries, stored_first=None):
    with zipfile.ZipFile(path, "w") as z:
        if stored_first:
            name, data = stored_first
            info = zipfile.ZipInfo(name, STAMP)
            info.compress_type = zipfile.ZIP_STORED
            z.writestr(info, data)
        for name, data in entries:
            info = zipfile.ZipInfo(name, STAMP)
            info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, data)


DOCX_TYPES = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/><Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/></Types>"""
DOCX_RELS = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"""
DOCX_DOC_RELS = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/></Relationships>"""
DOCX_STYLES = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:name w:val="Normal"/></w:style><w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/><w:basedOn w:val="Normal"/><w:next w:val="Normal"/><w:pPr><w:outlineLvl w:val="0"/></w:pPr><w:rPr><w:b/><w:sz w:val="32"/></w:rPr></w:style></w:styles>"""


def docx(path, pars):
    body = []
    for p in pars:
        ppr = '<w:pPr><w:pStyle w:val="Heading1"/></w:pPr>' if is_heading(p) else ""
        rs = []
        for text, italic in runs(p):
            rpr = "<w:rPr><w:i/></w:rPr>" if italic else ""
            rs.append(f'<w:r>{rpr}<w:t xml:space="preserve">{escape(text)}</w:t></w:r>')
        body.append(f"<w:p>{ppr}{''.join(rs)}</w:p>")
    doc = (
        '<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n'
        '<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>'
        + "".join(body)
        + '<w:sectPr><w:pgSz w:w="12240" w:h="15840"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="720" w:footer="720" w:gutter="0"/></w:sectPr></w:body></w:document>'
    )
    write_zip(
        path,
        [
            ("[Content_Types].xml", DOCX_TYPES),
            ("_rels/.rels", DOCX_RELS),
            ("word/_rels/document.xml.rels", DOCX_DOC_RELS),
            ("word/document.xml", doc),
            ("word/styles.xml", DOCX_STYLES),
        ],
    )


ODT_MANIFEST = """<?xml version="1.0" encoding="UTF-8"?>
<manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0" manifest:version="1.2"><manifest:file-entry manifest:full-path="/" manifest:media-type="application/vnd.oasis.opendocument.text"/><manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/><manifest:file-entry manifest:full-path="styles.xml" manifest:media-type="text/xml"/></manifest:manifest>"""
ODT_NS = (
    'xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" '
    'xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" '
    'xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" '
    'xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" office:version="1.2"'
)
ODT_STYLES = f"""<?xml version="1.0" encoding="UTF-8"?>
<office:document-styles {ODT_NS}><office:styles><style:style style:name="Standard" style:family="paragraph"/><style:style style:name="Heading_20_1" style:display-name="Heading 1" style:family="paragraph" style:parent-style-name="Standard" style:default-outline-level="1"><style:text-properties fo:font-weight="bold"/></style:style></office:styles></office:document-styles>"""


def odt(path, pars):
    body = []
    for p in pars:
        spans = "".join(
            f'<text:span text:style-name="T1">{escape(t)}</text:span>' if it else escape(t) for t, it in runs(p)
        )
        if is_heading(p):
            body.append(f'<text:h text:style-name="Heading_20_1" text:outline-level="1">{spans}</text:h>')
        else:
            body.append(f'<text:p text:style-name="Standard">{spans}</text:p>')
    content = (
        f'<?xml version="1.0" encoding="UTF-8"?>\n<office:document-content {ODT_NS}>'
        '<office:automatic-styles><style:style style:name="T1" style:family="text"><style:text-properties fo:font-style="italic"/></style:style></office:automatic-styles>'
        "<office:body><office:text>" + "".join(body) + "</office:text></office:body></office:document-content>"
    )
    write_zip(
        path,
        [("META-INF/manifest.xml", ODT_MANIFEST), ("content.xml", content), ("styles.xml", ODT_STYLES)],
        stored_first=("mimetype", "application/vnd.oasis.opendocument.text"),
    )


def rtf_text(s):
    out = []
    for ch in s:
        c = ord(ch)
        if ch in "\\{}":
            out.append("\\" + ch)
        elif c < 128:
            out.append(ch)
        else:
            if c > 0xFFFF:
                c -= 0x10000
                hi, lo = 0xD800 + (c >> 10), 0xDC00 + (c & 0x3FF)
                out.append(f"\\u{hi - 65536}?\\u{lo - 65536}?")
            else:
                out.append(f"\\u{c if c < 32768 else c - 65536}?")
    return "".join(out)


def rtf(path, pars):
    out = ["{\\rtf1\\ansi\\ansicpg1252\\deff0\\uc1{\\fonttbl{\\f0\\froman Serif;}}{\\stylesheet{\\s0 Normal;}{\\s1\\b\\fs32 heading 1;}}\n"]
    for p in pars:
        if is_heading(p):
            out.append("\\pard\\plain\\s1\\outlinelevel0\\b\\fs32 " + rtf_text(p) + "\\par\n")
        else:
            body = "".join("{\\i " + rtf_text(t) + "}" if it else rtf_text(t) for t, it in runs(p))
            out.append("\\pard\\plain\\s0 " + body + "\\par\n")
    out.append("}\n")
    with open(path, "w", encoding="ascii", newline="\n") as f:
        f.write("".join(out))


def html_page(path, pars, title):
    """A serial's chapter page: the chapters in an <article>, inside a page with navigation, a
    header and a footer written for the test (the import leaves those out)."""
    body = []
    for p in pars:
        text = "".join(f"<em>{escape(t)}</em>" if it else escape(t) for t, it in runs(p))
        body.append(f"<h2>{text}</h2>" if is_heading(p) else f"<p>{text}</p>")
    page = (
        "<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\"><title>"
        + escape(title)
        + "</title><style>body { font-family: serif; }</style><script>var visits = 0;</script></head>\n<body>\n"
        + "<header><p>A serial in weekly parts</p></header>\n<nav><a href=\"index.html\">Contents</a> <a href=\"part-2.html\">Next part</a></nav>\n"
        + "<article>\n"
        + "\n".join(body)
        + "\n</article>\n<footer><p>Posted for readers of the serial.</p></footer>\n</body></html>\n"
    )
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write(page)


def opml(path, pars, title):
    """The novel's chapters as an outline: three volumes of chapters, each chapter's first
    paragraph as its note (written for the test from the public-domain text)."""
    chapters = []
    for i, p in enumerate(pars):
        if is_heading(p):
            first = next((q for q in pars[i + 1:] if not is_heading(q) and not q.startswith("[Illustration")), "")
            chapters.append((p.rstrip("]").rstrip("."), ITALIC.sub(r"\1", first)))
    third = (len(chapters) + 2) // 3
    out = ['<?xml version="1.0" encoding="UTF-8"?>', '<opml version="2.0">', f"<head><title>{escape(title)}</title></head>", "<body>"]
    for v in range(3):
        out.append(f'  <outline text="Volume {v + 1}">')
        for name, note in chapters[v * third:(v + 1) * third]:
            out.append(f'    <outline text="{escape(name, {chr(34): "&quot;"})}" _note="{escape(note, {chr(34): "&quot;"})}"/>')
        out.append("  </outline>")
    out += ["</body>", "</opml>", ""]
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(out))


def read(name):
    with open(os.path.join(ROOT, "novels", name), encoding="utf-8") as f:
        return f.read()


def main():
    pride = paragraphs(read("pride-and-prejudice.txt"))
    os.makedirs(os.path.join(ROOT, "documents"), exist_ok=True)
    docx(os.path.join(ROOT, "documents", "pride-and-prejudice.docx"), pride)
    odt(os.path.join(ROOT, "documents", "pride-and-prejudice.odt"), pride)
    rtf(os.path.join(ROOT, "documents", "pride-and-prejudice.rtf"), pride)
    # A web serial's page (IMP-7): the first five chapters.
    starts = [i for i, p in enumerate(pride) if is_heading(p)]
    os.makedirs(os.path.join(ROOT, "web"), exist_ok=True)
    html_page(os.path.join(ROOT, "web", "pride-and-prejudice-part-1.html"), pride[starts[0]:starts[5]], "Pride and Prejudice, part 1")
    # An outline (IMP-8).
    os.makedirs(os.path.join(ROOT, "outlines"), exist_ok=True)
    opml(os.path.join(ROOT, "outlines", "pride-and-prejudice.opml"), pride, "Pride and Prejudice")

    # The v0.1 test set: Great Expectations in six files of one draft, about ten chapters each.
    great = paragraphs(read("great-expectations.txt"))
    starts = [i for i, p in enumerate(great) if is_heading(p)]
    cuts = [0] + [starts[n] for n in (10, 20, 30, 40, 50)] + [len(great)]
    folder = os.path.join(ROOT, "document-set")
    os.makedirs(folder, exist_ok=True)
    for n in range(6):
        docx(os.path.join(folder, f"Great Expectations (First Draft) {n + 1}.docx"), great[cuts[n]:cuts[n + 1]])


if __name__ == "__main__":
    main()
