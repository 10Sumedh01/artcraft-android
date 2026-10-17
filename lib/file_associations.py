"""Android file associations (``ACTION_VIEW`` / ``ACTION_SEND`` intent filters) for the ArtCraft suite.

A single source of truth shared by three callers:

* ``lib/build-common.sh`` (the launcher generator, for the 11 generated apps),
* ``adapters/photocraft/prepare.sh`` (the PhotoCraft adapter),
* the custom-adapter branch in ``lib/build-common.sh`` (the VectorCraft adapter).

For each app we list the *document* extensions it offers to open. Standard extensions are mapped to
the MIME type Android resolves for them (that is what actually drives the "Open with" chooser for
``content://`` URIs); custom project formats have no standard MIME and are matched by path pattern
only. ``intent_filter_toml`` renders the whole ``intent_filter = [...]`` value so every launcher
advertises exactly the same associations.
"""

import re

# Extension (lower-case, no dot) -> MIME type. Only well-known, specific types are listed: mapping a
# niche extension to a broad type (e.g. ``otio`` -> ``application/json``) would make the app a
# candidate for every JSON/XML file, so those are deliberately omitted and associated by extension.
EXT_MIME = {
    # Word-processing
    "docx": "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    "docm": "application/vnd.ms-word.document.macroEnabled.12",
    "dotx": "application/vnd.openxmlformats-officedocument.wordprocessingml.template",
    "dotm": "application/vnd.ms-word.template.macroEnabled.12",
    "doc": "application/msword",
    "dot": "application/msword",
    "rtf": "application/rtf",
    "odt": "application/vnd.oasis.opendocument.text",
    "txt": "text/plain",
    "md": "text/markdown",
    "markdown": "text/markdown",
    # Spreadsheets
    "xlsx": "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    "xlsm": "application/vnd.ms-excel.sheet.macroEnabled.12",
    "xltx": "application/vnd.openxmlformats-officedocument.spreadsheetml.template",
    "xltm": "application/vnd.ms-excel.template.macroEnabled.12",
    "xls": "application/vnd.ms-excel",
    "xlt": "application/vnd.ms-excel",
    "xlsb": "application/vnd.ms-excel.sheet.binary.macroEnabled.12",
    "ods": "application/vnd.oasis.opendocument.spreadsheet",
    "csv": "text/csv",
    "tsv": "text/tab-separated-values",
    "tab": "text/tab-separated-values",
    # Presentations
    "pptx": "application/vnd.openxmlformats-officedocument.presentationml.presentation",
    "potx": "application/vnd.openxmlformats-officedocument.presentationml.template",
    "ppsx": "application/vnd.openxmlformats-officedocument.presentationml.slideshow",
    # Page layout / print
    "pdf": "application/pdf",
    "idml": "application/vnd.adobe.indesign-idml-package",
    "indd": "application/x-indesign",
    "indt": "application/x-indesign",
    # Vector graphics
    "svg": "image/svg+xml",
    "svgz": "image/svg+xml",
    "ai": "application/postscript",
    "ait": "application/postscript",
    "eps": "application/postscript",
    "dxf": "image/vnd.dxf",
    "emf": "image/emf",
    "wmf": "image/wmf",
    # Raster / photographic
    "png": "image/png",
    "jpg": "image/jpeg",
    "jpeg": "image/jpeg",
    "gif": "image/gif",
    "webp": "image/webp",
    "tif": "image/tiff",
    "tiff": "image/tiff",
    "bmp": "image/bmp",
    "tga": "image/x-tga",
    "ico": "image/x-icon",
    "qoi": "image/qoi",
    "exr": "image/x-exr",
    "hdr": "image/vnd.radiance",
    "heic": "image/heic",
    "heif": "image/heif",
    "avif": "image/avif",
    "jxl": "image/jxl",
    "psd": "image/vnd.adobe.photoshop",
    "psb": "image/vnd.adobe.photoshop",
    "psdt": "image/vnd.adobe.photoshop",
    "pdn": "image/x-paintnet",
    "ora": "image/openraster",
    "kra": "application/x-krita",
    "krz": "application/x-krita",
    # Camera raw
    "dng": "image/x-adobe-dng",
    "cr2": "image/x-canon-cr2",
    "cr3": "image/x-canon-cr3",
    "nef": "image/x-nikon-nef",
    "nrw": "image/x-nikon-nrw",
    "arw": "image/x-sony-arw",
    "pef": "image/x-pentax-pef",
    "orf": "image/x-olympus-orf",
    "rw2": "image/x-panasonic-rw2",
    "raf": "image/x-fujifilm-raf",
    "rwl": "image/x-leica-rwl",
    "srw": "image/x-samsung-srw",
    "crw": "image/x-canon-crw",
    "mrw": "image/x-minolta-mrw",
    "x3f": "image/x-sigma-x3f",
    "kdc": "image/x-kodak-kdc",
    "erf": "image/x-epson-erf",
    "3fr": "image/x-hasselblad-3fr",
    # Video
    "mp4": "video/mp4",
    "m4v": "video/x-m4v",
    "mov": "video/quicktime",
    "mkv": "video/x-matroska",
    "webm": "video/webm",
    "avi": "video/x-msvideo",
    "mxf": "application/mxf",
    "mts": "video/mp2t",
    "m2ts": "video/mp2t",
    "m2t": "video/mp2t",
    "ts": "video/mp2t",
    "mpg": "video/mpeg",
    "mpeg": "video/mpeg",
    "vob": "video/mpeg",
    "mod": "video/mpeg",
    "tod": "video/mp2t",
    "m2v": "video/mpeg",
    "m1v": "video/mpeg",
    "mpv": "video/mpeg",
    "3gp": "video/3gpp",
    "y4m": "video/x-yuv4mpeg",
    # Audio
    "wav": "audio/x-wav",
    "wave": "audio/x-wav",
    "bwf": "audio/x-wav",
    "aif": "audio/x-aiff",
    "aiff": "audio/x-aiff",
    "aifc": "audio/x-aiff",
    "mp3": "audio/mpeg",
    "mp2": "audio/mpeg",
    "m4a": "audio/mp4",
    "m4b": "audio/mp4",
    "aac": "audio/aac",
    "flac": "audio/flac",
    "ogg": "audio/ogg",
    "oga": "audio/ogg",
    "opus": "audio/opus",
    "caf": "audio/x-caf",
    "mid": "audio/midi",
    "midi": "audio/midi",
    "smf": "audio/midi",
    # Captions
    "srt": "application/x-subrip",
    "vtt": "text/vtt",
}

# App -> the document extensions it offers to open. Kept to the app's own document formats (a
# spreadsheet does not claim every .txt, an image editor does not claim PDFs) so the chooser stays
# useful; the raw, order-sensitive extension lists live in the app sources this was derived from.
ASSOCIATIONS = {
    "wordcraft": ["docx", "docm", "dotx", "dotm", "doc", "dot", "rtf", "odt", "txt", "md", "markdown"],
    "gridcraft": ["xlsx", "xlsm", "xltx", "xltm", "xls", "xlt", "xlsb", "ods", "csv", "tsv"],
    "deckcraft": ["deckcraft", "pptx", "potx", "ppsx"],
    "designcraft": ["designcraft", "idml", "indd", "indt", "pdf"],
    "vectorcraft": ["vectorcraft", "drawcraft", "svg", "svgz", "ai", "ait", "eps", "pdf", "dxf", "emf", "wmf", "vctemplate"],
    "photocraft": [
        "pcraft", "psd", "psb", "psdt", "pdn", "ora", "kra", "krz",
        "png", "jpg", "jpeg", "gif", "webp", "tif", "tiff", "bmp", "tga", "ico", "qoi", "exr", "hdr",
        "heic", "heif", "avif", "svg", "svgz",
        "dng", "cr2", "cr3", "nef", "nrw", "arw", "pef", "orf", "rw2", "raf",
    ],
    "lightcraft": [
        "jpg", "jpeg", "png", "tif", "tiff", "webp", "gif", "bmp", "heic", "heif", "avif", "jxl",
        "dng", "cr2", "cr3", "nef", "nrw", "arw", "raf", "orf", "rw2", "rwl", "pef", "srw", "crw",
        "mrw", "x3f", "kdc", "erf", "3fr", "psd", "lrcat",
    ],
    "filmcraft": [
        "fcproj",
        "mp4", "m4v", "mov", "mkv", "webm", "avi", "mxf", "mts", "m2ts", "m2t", "ts", "mpg",
        "mpeg", "vob", "mod", "tod", "m2v", "m1v", "mpv", "3gp", "y4m",
        "wav", "wave", "bwf", "aif", "aiff", "mp3", "mp2", "m4a", "aac", "flac", "ogg", "opus",
        "srt", "vtt", "scc", "edl", "fcpxml", "otio", "aaf", "omf",
    ],
    "soundcraft": [
        "scraft", "mid", "midi", "smf",
        "wav", "wave", "bwf", "aif", "aiff", "aifc", "flac", "mp3", "ogg", "oga", "m4a", "m4b",
        "aac", "caf", "mp4", "mkv", "webm",
    ],
    "pdfcraft": ["pdf"],
    "figcraft": ["figcraft", "fig"],
    "effectcraft": ["ecproj", "ecprojx", "ectemplate"],
    "cadcraft": ["dwg", "dwt", "dxf"],
}

MAIN_FILTER = '{ actions = ["android.intent.action.MAIN"], categories = ["android.intent.category.LAUNCHER"] }'


def _mime_types(app):
    seen = []
    for ext in ASSOCIATIONS.get(app, []):
        mime = EXT_MIME.get(ext)
        if mime and mime not in seen:
            seen.append(mime)
    return seen


def intent_filter_toml(app):
    """Render the complete inline ``intent_filter = [...]`` *value* for ``app``.

    Starts with the MAIN/LAUNCHER filter the launcher needs and appends, when the app has file
    associations, the VIEW filters. Android's data matching applies every declared MIME type and
    every declared URI component in a single filter, so the MIME, the by-extension path pattern and
    the share targets are each their own filter rather than one combined element.
    """
    filters = [MAIN_FILTER]
    exts = ASSOCIATIONS.get(app, [])
    mimes = _mime_types(app)

    if mimes:
        data = ", ".join('{{ mime_type = "{0}" }}'.format(m) for m in mimes)
        filters.append(
            '{{ actions = ["android.intent.action.VIEW"], categories = ["android.intent.category.DEFAULT"], data = [{0}] }}'.format(data)
        )
        # Let a browser hand a remote document straight to the app too (content:// links already
        # match above through their MIME type).
        web = ", ".join(
            '{{ scheme = "{0}", mime_type = "{1}" }}'.format(scheme, mime)
            for mime in mimes
            for scheme in ("http", "https")
        )
        filters.append(
            '{{ actions = ["android.intent.action.VIEW"], categories = ["android.intent.category.DEFAULT", "android.intent.category.BROWSABLE"], data = [{0}] }}'.format(web)
        )
        share = ", ".join('{{ mime_type = "{0}" }}'.format(m) for m in mimes)
        filters.append(
            '{{ actions = ["android.intent.action.SEND", "android.intent.action.SEND_MULTIPLE"], categories = ["android.intent.category.DEFAULT"], data = [{0}] }}'.format(share)
        )

    if exts:
        # Matched by file name for providers that report a generic type (application/octet-stream)
        # for the app's own project formats. `.` is Android's "any character" wildcard, so no
        # backslash escaping is involved.
        data = ", ".join('{{ scheme = "{0}", path_pattern = ".*.{1}" }}'.format(scheme, ext) for ext in exts for scheme in ("file", "content"))
        filters.append(
            '{{ actions = ["android.intent.action.VIEW"], categories = ["android.intent.category.DEFAULT"], data = [{0}] }}'.format(data)
        )

    return "[" + ", ".join(filters) + "]"


def patch_cargo_toml(path, app):
    """Replace the single ``intent_filter = [...]`` line in a copied launcher manifest.

    Returns True when the file was rewritten and False when the app has no associations (or the
    manifest has no intent_filter line to patch).
    """
    path = str(path)
    with open(path, "r", encoding="utf-8") as handle:
        text = handle.read()
    if "intent_filter" not in text:
        return False
    value = intent_filter_toml(app)
    new_text, count = re.subn(r"(?m)^intent_filter = .*$", lambda _m: "intent_filter = " + value, text, count=1)
    if count == 0:
        return False
    if new_text != text:
        with open(path, "w", encoding="utf-8") as handle:
            handle.write(new_text)
    return True
