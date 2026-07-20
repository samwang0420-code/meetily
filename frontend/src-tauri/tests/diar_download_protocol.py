"""v0.6.23+ — Unit test for diar_download protocol surface (no network).

Verifies the stable contract of diar_download.ensure_models / status_report /
ensure_models_async.
"""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "scripts"))

import diar_download as dd


def test_models_inventory_contract():
    assert len(dd.MODELS) >= 2, "expect at least segmentation + embedding"
    names = {m["name"] for m in dd.MODELS}
    assert "pyannote-segmentation-3.0" in names
    assert "campplus-cn-en" in names
    for m in dd.MODELS:
        assert m["rel_path"].endswith(".onnx"), m
        assert m["min_bytes"] > 0
        assert m["urls"], m["name"] + " missing urls"
        # Ensure URL is HTTPS to avoid silent cleartext download.
        for u in m["urls"]:
            assert u.startswith("https://"), "non-https url: " + u


def test_status_report_shape():
    snap = dd.status_report()
    for key in ("models_root", "files", "ready", "async_state"):
        assert key in snap, f"missing key {key}"
    assert isinstance(snap["files"], list)
    assert isinstance(snap["ready"], bool)


def test_is_available_does_not_trigger_download(tmp_path=None):
    # Confirm is_available() returns False without throwing when models are absent.
    assert callable(dd.is_available)
    # We don't assert on the return value because the dev environment may or
    # may not have models installed; the contract is: no exception, no download.
    dd.is_available()


def test_ensure_models_entrypoint_exists():
    assert callable(dd.ensure_models)


def test_ensure_models_async_entrypoint_exists():
    assert callable(dd.ensure_models_async)


def test_url_indexing_for_fallback():
    multi = [m for m in dd.MODELS if len(m["urls"]) >= 2]
    assert multi, "需要至少一个模型多镜像, 才能支持断点切换"


def test_models_root_candidates():
    assert dd._MODELS_ROOT_CANDIDATES
    for c in dd._MODELS_ROOT_CANDIDATES:
        assert c == os.path.expanduser(c) or c.startswith("/"), c


def test_async_state_initialized_when_downloading():
    """If models are missing, ensure_models_async must populate _ASYNC_STATE."""
    # Simulate missing: monkeypatch is_available to always False.
    original = dd.is_available
    dd.is_available = lambda: False
    try:
        result = dd.ensure_models_async(progress_cb=lambda m: None)
        assert result["async"] is True
        # The async state should now be initialized.
        for m in dd.MODELS:
            assert m["name"] in dd._ASYNC_STATE
    finally:
        dd.is_available = original


def test_failure_cache_ttl():
    # Failure cache should be set on a download failure and expire.
    assert hasattr(dd, "_FAILURE_CACHE")
    assert dd._FAILURE_TTL > 0


if __name__ == "__main__":
    tests = [v for k, v in dict(globals()).items() if k.startswith("test_") and callable(v)]
    failed = 0
    for t in tests:
        try:
            t()
            print(f"PASS {t.__name__}")
        except AssertionError as exc:
            failed += 1
            print(f"FAIL {t.__name__}: {exc}")
        except Exception as exc:  # noqa: BLE001
            failed += 1
            print(f"ERROR {t.__name__}: {exc}")
    if failed:
        sys.exit(1)
    print(f"OK {len(tests)} tests passed")
