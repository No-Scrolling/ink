#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">=3.11"
# dependencies = ["pillow"]
# ///
"""Generate unsigned, synthetic PKPass attachments for manual rendering checks."""
import argparse
import hashlib
import io
import json
from pathlib import Path
import zipfile

from PIL import Image, ImageDraw


def field(key, label, value, **extra):
    return dict(key=key, label=label, value=value, **extra)


def artwork(role, shape="wide", scale=2):
    width, height = {"wide": (350, 110), "square": (100, 100), "portrait": (100, 180), "padded": (350, 110)}[shape]
    image = Image.new("RGBA", (width * scale, height * scale), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    box = (0, 0, image.width - 1, image.height - 1)
    if shape == "padded":
        box = (image.width // 3, 0, image.width * 2 // 3, image.height - 1)
    draw.rectangle(box, fill="#194b82", outline="#ffffff", width=3 * scale)
    draw.line((box[0], box[1], box[2], box[3]), fill="#527fae", width=scale)
    draw.line((box[0], box[3], box[2], box[1]), fill="#527fae", width=scale)
    draw.text(((box[0] + box[2]) // 2, image.height // 2), f"TEST {role.upper()}", fill="white", anchor="mm", font_size=12 * scale)
    output = io.BytesIO()
    image.save(output, format="PNG")
    return output.getvalue()


def generate(output):
    output.mkdir(parents=True, exist_ok=True)
    cases = []

    def add(name, style, groups=None, *, code="QR", images=None, note="", **extra):
        cases.append(dict(name=name, style=style, groups=groups or {}, code=code,
                          images=images if images is not None else [("icon", "square", 2)], note=note, extra=extra))

    route = dict(primaryFields=[field("from", "Origin", "AAA"), field("to", "Destination", "BBB")],
                 secondaryFields=[field("date", "Date", "29 Sep 2026"), field("seat", "Seat", "12A")],
                 auxiliaryFields=[field("class", "Class", "Standard"), field("zone", "Zone", "B")])
    member = dict(secondaryFields=[field("name", "Name", "Test Member"), field("number", "Member number", "000000001234567890")])
    for mode in ["Train", "Air", "Bus", "Boat", "Generic"]:
        add(f"boarding-{mode.lower()}", "boardingPass", {**route, "transitType": f"PKTransitType{mode}"}, code="Aztec", note="Paired route, three field groups, centred bottom barcode.")
    add("store-banner", "storeCard", member, images=[("icon", "square", 2), ("strip", "wide", 2)], note="Full-width strip; 18-digit member number remains readable.")
    add("event-poster", "eventTicket", dict(primaryFields=[field("event", "Event", "TEST LIVE")], secondaryFields=[field("date", "Date", "29 Sep 2026"), field("seat", "Seat", "A12")]), images=[("icon", "square", 2), ("thumbnail", "portrait", 2)], note="Portrait artwork must keep its aspect ratio.")
    add("coupon-offer", "coupon", dict(primaryFields=[field("offer", "Offer", "20% OFF")], auxiliaryFields=[field("terms", "Applies to", "TEST ITEMS ONLY")]), code="PDF417")
    add("generic-membership", "generic", member, code="Code128")
    for code in ["QR", "Aztec", "PDF417", "Code128"]:
        add(f"barcode-{code.lower()}", "generic", dict(secondaryFields=[field("format", "Barcode", code)]), code=code, note="Correct shape and uncropped barcode; TEST caption below.")
    add("no-barcode", "storeCard", member, code=None, note="Explicit no-supported-barcode message; no crash.")
    add("unsupported-barcode", "generic", member, code="Unsupported", note="Unknown format is omitted gracefully.")
    add("no-artwork", "storeCard", member, images=[], note="Text preview card and detail page remain usable.")
    add("empty-primary", "storeCard", {**member, "primaryFields": [field("empty", "", ""), field("spaces", " ", " \n ")]}, images=[("strip", "padded", 3)], note="No blank primary-field gap; transparent strip padding is preserved.")
    add("odd-field-count", "generic", dict(secondaryFields=[field(str(i), f"Field {i}", f"Value {i}") for i in range(1, 6)]), note="Five fields produce a final unpaired row.")
    add("long-values", "generic", dict(secondaryFields=[field("name", "Long label for a member's full name", "Alexandra Example-With-A-Very-Long-Surname"), field("number", "Member number", "00000000111111112222222233333333")]), note="Long fields wrap without overlap, loss or clipping.")
    add("japanese-emoji", "eventTicket", dict(primaryFields=[field("title", "公演", "音楽祭 🎵")], secondaryFields=[field("name", "お名前", "テスト 太郎"), field("seat", "座席", "東京 🗼 A列")]), note="Japanese glyphs and emoji render correctly.")
    add("arabic-rtl", "generic", dict(secondaryFields=[field("name", "الاسم", "بطاقة اختبار"), field("id", "Number", "TEST-1234")]), note="Inspect RTL shaping and mixed-script alignment.")
    add("info-links", "storeCard", {**member, "backFields": [field("links", "Information", 'Read <a href="https://example.com/terms">test terms</a><br>Then contact <a href="mailto:wallet-test@example.com">support</a>.'), field("plain", "Website", "https://example.com/help")]}, note="Info appears only in Passes; linked text is underlined and tappable.")
    add("background-footer", "eventTicket", dict(secondaryFields=[field("event", "Event", "TEST SHOW")]), images=[("background", "wide", 2), ("footer", "wide", 2)], note="Background fallback and footer both render; scroll if required.")
    add("all-artwork-roles", "generic", member, images=[("icon", "square", 2), ("logo", "wide", 2), ("strip", "wide", 2), ("background", "wide", 2), ("thumbnail", "square", 2), ("footer", "wide", 2)], note="Strip takes priority over background; artwork never overlaps fields.")
    add("resolution-variants", "storeCard", member, images=[("strip", "wide", scale) for scale in [1, 2, 3]], note="Highest-resolution artwork chosen with consistent aspect ratio.")
    add("many-fields", "generic", dict(secondaryFields=[field(str(i), f"Label {i}", f"TEST value {i}") for i in range(1, 13)]), note="Long pass scrolls; all fields and barcode remain reachable.")
    add("header-only", "generic", dict(headerFields=[field("points", "Points", "12345")]), note="Header-only points remain visible as a detail row.")
    add("numeric-date-values", "coupon", dict(secondaryFields=[field("points", "Points", 12345), field("date", "Expiry", "2026-12-31T23:59:00Z", dateStyle="PKDateStyleMedium", timeStyle="PKDateStyleNone")]), note="Numeric value is preserved; expiry uses the requested medium date format.")
    add("multiline-values", "generic", dict(secondaryFields=[field("address", "Address", "1 Example Street\nTest Town\nTEST 001"), field("notes", "Notes", "Line one\nLine two")]), note="Explicit newlines are preserved without overlap.")
    add("legacy-barcode-encoding", "storeCard", member, code="QR", encoding="iso-8859-1", payload="TEST-Caf\u00e9", note="Unsupported non-ASCII legacy encoding is omitted rather than misencoded.")

    catalog = []
    for index, case in enumerate(cases, 1):
        name = f"{index:02d}-TEST-{case['name']}"
        fields = case["groups"]
        data = dict(formatVersion=1, passTypeIdentifier="pass.test.ink.fixtures", teamIdentifier="INKTEST000",
                    serialNumber=name, organizationName="Ink Test Fixtures",
                    description=f"TEST {index:02d} {case['name'].replace('-', ' ')}", logoText=f"TEST {index:02d}",
                    foregroundColor="rgb(255,255,255)", backgroundColor="rgb(25,75,130)",
                    labelColor="rgb(220,220,220)", **{case["style"]: fields})
        if case["code"]:
            data["barcodes"] = [dict(format=f"PKBarcodeFormat{case['code']}", message=case["extra"].get("payload", f"INK-TEST-{index:02d}-NOT-VALID"),
                                     messageEncoding=case["extra"].get("encoding", "utf-8"), altText=f"TEST {index:02d} — NOT VALID")]
        entries = {"pass.json": json.dumps(data, ensure_ascii=False).encode()}
        for role, shape, scale in case["images"]:
            entries[f"{role}{f'@{scale}x' if scale > 1 else ''}.png"] = artwork(role, shape, scale)
        entries["manifest.json"] = json.dumps({key: hashlib.sha1(value).hexdigest() for key, value in entries.items()}).encode()
        path = output / f"{name}.pkpass"
        with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as archive:
            for key, value in entries.items():
                entry = zipfile.ZipInfo(key, date_time=(2026, 9, 29, 0, 0, 0))
                entry.compress_type = zipfile.ZIP_DEFLATED
                archive.writestr(entry, value)
        catalog.append(dict(file=path.name, style=case["style"], barcode=case["code"], expected=case["note"]))
    return catalog


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--catalog", type=Path, required=True)
    args = parser.parse_args()
    catalog = generate(args.output)
    args.catalog.parent.mkdir(parents=True, exist_ok=True)
    args.catalog.write_text(json.dumps(catalog, indent=2) + "\n")
    print(f"Generated {len(catalog)} unsigned test passes in {args.output}")
