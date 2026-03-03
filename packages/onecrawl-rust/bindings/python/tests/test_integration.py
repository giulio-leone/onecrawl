"""Integration test: Browser → Parser → Crypto → Storage pipeline."""
import json
import tempfile
import os
import pytest


def test_full_pipeline():
    """Chain browser + parser + crypto + storage in one flow."""
    from onecrawl import Browser, Store
    from onecrawl.crypto import encrypt, decrypt, generate_pkce, generate_totp, verify_totp
    from onecrawl.parser import parse_accessibility_tree, query_selector, extract_text, extract_links

    browser = Browser.launch(headless=True)
    tmp_dir = tempfile.mkdtemp(prefix="onecrawl-int-")
    store_path = os.path.join(tmp_dir, "test-store")

    try:
        # Step 1: Browser navigates
        browser.goto("https://example.com")
        title = browser.get_title()
        assert title == "Example Domain"

        # Step 2: Extract HTML content
        html = browser.content()
        assert len(html) > 100

        # Step 3: Parser — accessibility tree
        tree = parse_accessibility_tree(html)
        assert len(tree) > 0
        assert "Example Domain" in tree

        # Step 4: Parser — querySelector
        results = query_selector(html, "h1")
        parsed = json.loads(results)
        assert len(parsed) > 0

        # Step 5: Parser — extractText
        text = extract_text(html)
        assert "Example Domain" in text

        # Step 6: Parser — extractLinks
        links = extract_links(html)
        assert len(links) > 0
        iana_links = [l for l in links if "iana.org" in l[0]]
        assert len(iana_links) > 0
        assert iana_links[0][2] is True  # is_external

        # Step 7: Crypto — encrypt/decrypt HTML
        password = "integration-test-password-2024"
        html_bytes = html.encode("utf-8")
        encrypted = encrypt(html_bytes, password)
        assert len(encrypted) > len(html_bytes)
        decrypted = decrypt(encrypted, password)
        assert bytes(decrypted) == html_bytes

        # Step 8: Storage — persist metadata
        store = Store(store_path, "store-password-2024")
        store.set("page:url", "https://example.com")
        store.set("page:title", "Example Domain")
        store.set("page:html_length", str(len(html)))

        assert store.get("page:url") == "https://example.com"
        assert store.get("page:title") == "Example Domain"
        assert store.contains("page:html_length")

        # Step 9: Crypto — PKCE
        verifier, challenge = generate_pkce()
        assert len(verifier) >= 43
        assert len(challenge) > 0
        store.set("auth:pkce_verifier", verifier)

        # Step 10: Crypto — TOTP
        secret = "JBSWY3DPEHPK3PXP"
        code = generate_totp(secret)
        assert len(code) == 6
        assert code.isdigit()
        assert verify_totp(secret, code) is True
        store.set("auth:last_totp", code)

        # Step 11: Screenshot + store metadata
        png = browser.screenshot()
        assert len(png) > 1000
        store.set("screenshot:size", str(len(png)))
        store.flush()

        # Step 12: Full verification
        keys = store.keys()
        assert len(keys) >= 6

        url = store.get("page:url")
        assert url == "https://example.com"

        screenshot_size = int(store.get("screenshot:size"))
        assert screenshot_size > 1000

        pkce = store.get("auth:pkce_verifier")
        assert len(pkce) >= 43

        totp = store.get("auth:last_totp")
        assert len(totp) == 6 and totp.isdigit()

    finally:
        browser.close()
        import shutil
        shutil.rmtree(tmp_dir, ignore_errors=True)
