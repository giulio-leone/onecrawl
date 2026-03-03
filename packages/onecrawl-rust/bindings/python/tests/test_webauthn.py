"""Tests for WebAuthn virtual authenticator simulation."""

import json
import pytest
from onecrawl import Browser


@pytest.fixture(scope="module")
def browser():
    b = Browser.launch(headless=True)
    yield b
    b.close()


class TestWebauthn:
    def test_enable_virtual_authenticator(self, browser):
        browser.goto("data:text/html,<h1>WebAuthn</h1>")
        config = json.dumps({
            "id": "test-auth",
            "protocol": "ctap2",
            "transport": "internal",
            "has_resident_key": True,
            "has_user_verification": True,
            "is_user_verified": True,
        })
        browser.enable_virtual_authenticator(config)

    def test_get_virtual_credentials_empty(self, browser):
        raw = browser.get_virtual_credentials()
        creds = json.loads(raw)
        assert isinstance(creds, list)
        assert len(creds) == 0

    def test_add_virtual_credential(self, browser):
        cred = json.dumps({
            "credential_id": "dGVzdC1jcmVk",
            "rp_id": "example.com",
            "user_handle": "dXNlcjE",
            "sign_count": 0,
        })
        browser.add_virtual_credential(cred)

    def test_get_virtual_credentials_has_one(self, browser):
        raw = browser.get_virtual_credentials()
        creds = json.loads(raw)
        assert len(creds) == 1
        assert creds[0]["credential_id"] == "dGVzdC1jcmVk"
        assert creds[0]["rp_id"] == "example.com"

    def test_add_multiple_credentials(self, browser):
        cred2 = json.dumps({
            "credential_id": "c2Vjb25k",
            "rp_id": "other.com",
            "user_handle": "dXNlcjI",
            "sign_count": 5,
        })
        browser.add_virtual_credential(cred2)
        raw = browser.get_virtual_credentials()
        creds = json.loads(raw)
        assert len(creds) == 2

    def test_get_webauthn_log(self, browser):
        raw = browser.get_webauthn_log()
        log = json.loads(raw)
        assert isinstance(log, list)

    def test_remove_virtual_credential(self, browser):
        removed = browser.remove_virtual_credential("dGVzdC1jcmVk")
        assert removed is True
        raw = browser.get_virtual_credentials()
        creds = json.loads(raw)
        assert len(creds) == 1
        assert creds[0]["credential_id"] == "c2Vjb25k"

    def test_remove_nonexistent_credential(self, browser):
        removed = browser.remove_virtual_credential("nonexistent")
        assert removed is False

    def test_disable_virtual_authenticator(self, browser):
        browser.disable_virtual_authenticator()

    def test_credentials_empty_after_disable(self, browser):
        raw = browser.get_virtual_credentials()
        creds = json.loads(raw)
        assert len(creds) == 0

    def test_enable_u2f_authenticator(self, browser):
        config = json.dumps({
            "id": "u2f-auth",
            "protocol": "u2f",
            "transport": "usb",
            "has_resident_key": False,
            "has_user_verification": False,
            "is_user_verified": False,
        })
        browser.enable_virtual_authenticator(config)
        browser.disable_virtual_authenticator()
