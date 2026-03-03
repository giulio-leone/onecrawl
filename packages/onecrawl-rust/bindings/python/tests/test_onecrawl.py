import pytest
import tempfile
import os


def test_encrypt_decrypt():
    from onecrawl.crypto import encrypt, decrypt
    data = b"Hello OneCrawl"
    ct = encrypt(data, "password")
    assert len(ct) > len(data)
    pt = decrypt(ct, "password")
    assert pt == data


def test_decrypt_wrong_password():
    from onecrawl.crypto import encrypt, decrypt
    ct = encrypt(b"secret", "correct")
    with pytest.raises(Exception):
        decrypt(ct, "wrong")


def test_derive_key():
    from onecrawl.crypto import derive_key
    key = derive_key("password", bytes(16))
    assert len(key) == 32


def test_derive_key_deterministic():
    from onecrawl.crypto import derive_key
    salt = bytes(16)
    k1 = derive_key("pw", salt)
    k2 = derive_key("pw", salt)
    assert k1 == k2


def test_generate_pkce():
    from onecrawl.crypto import generate_pkce
    verifier, challenge = generate_pkce()
    assert len(verifier) >= 43
    assert len(challenge) > 0
    assert verifier != challenge


def test_generate_totp():
    from onecrawl.crypto import generate_totp
    code = generate_totp("JBSWY3DPEHPK3PXP")
    assert len(code) == 6
    assert code.isdigit()


def test_verify_totp():
    from onecrawl.crypto import generate_totp, verify_totp
    secret = "JBSWY3DPEHPK3PXP"
    code = generate_totp(secret)
    assert verify_totp(secret, code)


def test_accessibility_tree():
    from onecrawl.parser import parse_accessibility_tree
    html = "<html><body><h1>Hello</h1><p>World</p></body></html>"
    tree = parse_accessibility_tree(html)
    assert "Hello" in tree


def test_query_selector():
    from onecrawl.parser import query_selector
    import json
    html = "<html><body><li>A</li><li>B</li></body></html>"
    result = json.loads(query_selector(html, "li"))
    assert len(result) == 2


def test_extract_text():
    from onecrawl.parser import extract_text
    html = "<html><body><p>Hello World</p></body></html>"
    text = extract_text(html)
    assert "Hello World" in text


def test_extract_links():
    from onecrawl.parser import extract_links
    html = '<html><body><a href="https://example.com">Link</a></body></html>'
    links = extract_links(html)
    assert len(links) == 1
    assert links[0][0] == "https://example.com"
    assert links[0][1] == "Link"
    assert links[0][2] == True


def test_store_set_get():
    from onecrawl import Store
    with tempfile.TemporaryDirectory() as d:
        s = Store(os.path.join(d, "db"), "pw")
        s.set("key", "value")
        assert s.get("key") == "value"


def test_store_get_missing():
    from onecrawl import Store
    with tempfile.TemporaryDirectory() as d:
        s = Store(os.path.join(d, "db"), "pw")
        assert s.get("nope") is None


def test_store_delete():
    from onecrawl import Store
    with tempfile.TemporaryDirectory() as d:
        s = Store(os.path.join(d, "db"), "pw")
        s.set("a", "b")
        assert s.delete("a")
        assert s.get("a") is None


def test_store_keys():
    from onecrawl import Store
    with tempfile.TemporaryDirectory() as d:
        s = Store(os.path.join(d, "db"), "pw")
        s.set("x", "1")
        s.set("y", "2")
        keys = s.keys()
        assert "x" in keys
        assert "y" in keys


def test_store_contains():
    from onecrawl import Store
    with tempfile.TemporaryDirectory() as d:
        s = Store(os.path.join(d, "db"), "pw")
        s.set("exists", "yes")
        assert s.contains("exists")
        assert not s.contains("nope")


def test_store_flush():
    from onecrawl import Store
    with tempfile.TemporaryDirectory() as d:
        s = Store(os.path.join(d, "db"), "pw")
        s.set("k", "v")
        s.flush()  # should not raise
