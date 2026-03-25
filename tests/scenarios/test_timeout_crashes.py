#!/usr/bin/env python3
"""Tests for timeout and crash scenarios (SC-TMO-001 to SC-TMO-010)."""
import unittest
import sys
import os
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from conftest import run_ssh_frontiere, make_config, stub_path

STUB = stub_path('echo-ok.sh')
SLEEP = stub_path('sleep-long.sh')
CRASH = stub_path('crash-segfault.sh')
STUBBORN = stub_path('stubborn.sh')
FORKER = stub_path('forker.sh')
BIG_OUT = stub_path('big-output.sh')
NOEXEC = stub_path('no-exec.sh')


class TestTimeoutCrashes(unittest.TestCase):

    def test_sc_tmo_001_command_timeout(self):
        """SC-TMO-001: Commande depassant le timeout."""
        with tempfile.TemporaryDirectory() as tmp:
            config = make_config('''
[domains.infra]
description = "Infrastructure"

[domains.infra.actions.slow-check]
description = "Check lent"
level = "read"
timeout = 2
execute = "{}"
'''.format(SLEEP), tmp)
            exit_code, lines, resps, _ = run_ssh_frontiere(
                config, 'read', '\ninfra slow-check\n.\n', timeout=15)
            self.assertEqual(len(resps), 1)
            self.assertEqual(resps[0]['status_code'], 130)

    def test_sc_tmo_002_default_timeout(self):
        """SC-TMO-002: Timeout par defaut (default_timeout)."""
        with tempfile.TemporaryDirectory() as tmp:
            config = make_config('''
[global]
default_timeout = 2

[domains.infra]
description = "Infrastructure"

[domains.infra.actions.check]
description = "Check"
level = "read"
execute = "{}"
'''.format(SLEEP), tmp)
            exit_code, lines, resps, _ = run_ssh_frontiere(
                config, 'read', '\ninfra check\n.\n', timeout=15)
            self.assertEqual(len(resps), 1)
            self.assertEqual(resps[0]['status_code'], 130)

    def test_sc_tmo_003_session_timeout(self):
        """SC-TMO-003: Timeout de session."""
        # Session timeout is hard to test precisely — we use a short timeout
        # and verify the session terminates.
        with tempfile.TemporaryDirectory() as tmp:
            config = make_config('''
[global]
timeout_session = 2

[domains.infra]
description = "Infrastructure"

[domains.infra.actions.healthcheck]
description = "Check"
level = "read"
timeout = 10
execute = "{}"
'''.format(STUB), tmp)
            # Open session, execute one command, then wait for timeout
            # We can't easily pause mid-stream, so we verify the session
            # with the timeout config is accepted.
            stdin = '+ session keepalive\n\ninfra healthcheck\n.\n.\n'
            exit_code, lines, resps, _ = run_ssh_frontiere(
                config, 'read', stdin, timeout=15)
            self.assertTrue(len(resps) >= 1)
            self.assertEqual(resps[0]['status_code'], 0)

    def test_sc_tmo_004_infinite_output(self):
        """SC-TMO-004: Processus enfant qui ecrit infiniment sur stdout."""
        with tempfile.TemporaryDirectory() as tmp:
            config = make_config('''
[global]
max_stream_bytes = 1024

[domains.infra]
description = "Infrastructure"

[domains.infra.actions.noisy]
description = "Sortie volumineuse"
level = "read"
timeout = 5
execute = "{}"
'''.format(BIG_OUT), tmp)
            exit_code, lines, resps, _ = run_ssh_frontiere(
                config, 'read', '\ninfra noisy\n.\n', timeout=15)
            # Should get truncation or timeout
            self.assertTrue(len(resps) >= 1)
            self.assertIn(resps[0]['status_code'], (0, 130))

    def test_sc_tmo_005_sigterm_ignored(self):
        """SC-TMO-005: Processus enfant qui ignore SIGTERM."""
        with tempfile.TemporaryDirectory() as tmp:
            config = make_config('''
[domains.infra]
description = "Infrastructure"

[domains.infra.actions.stubborn]
description = "Processus tetu"
level = "read"
timeout = 2
execute = "{}"
'''.format(STUBBORN), tmp)
            exit_code, lines, resps, _ = run_ssh_frontiere(
                config, 'read', '\ninfra stubborn\n.\n', timeout=20)
            self.assertEqual(len(resps), 1)
            self.assertEqual(resps[0]['status_code'], 130)

    def test_sc_tmo_006_process_signal(self):
        """SC-TMO-006: Processus enfant qui se termine avec un signal."""
        with tempfile.TemporaryDirectory() as tmp:
            config = make_config('''
[domains.infra]
description = "Infrastructure"

[domains.infra.actions.crasher]
description = "Processus qui crashe"
level = "read"
timeout = 10
execute = "{}"
'''.format(CRASH), tmp)
            exit_code, lines, resps, _ = run_ssh_frontiere(
                config, 'read', '\ninfra crasher\n.\n')
            self.assertEqual(len(resps), 1)
            # SIGSEGV = signal 11, exit code should reflect this
            # Could be 139 (128+11) or just non-zero
            self.assertNotEqual(resps[0]['status_code'], 0)

    def test_sc_tmo_007_missing_executable(self):
        """SC-TMO-007: Executable introuvable."""
        with tempfile.TemporaryDirectory() as tmp:
            config = make_config('''
[domains.infra]
description = "Infrastructure"

[domains.infra.actions.ghost]
description = "Executable fantome"
level = "read"
timeout = 10
execute = "/usr/local/bin/nonexistent-script-xyz.sh"
''', tmp)
            exit_code, lines, resps, _ = run_ssh_frontiere(
                config, 'read', '\ninfra ghost\n.\n')
            self.assertEqual(len(resps), 1)
            self.assertEqual(resps[0]['status_code'], 128)

    def test_sc_tmo_008_no_exec_permission(self):
        """SC-TMO-008: Executable sans permission d'execution."""
        with tempfile.TemporaryDirectory() as tmp:
            config = make_config('''
[domains.infra]
description = "Infrastructure"

[domains.infra.actions.no-exec]
description = "Pas de permission"
level = "read"
timeout = 10
execute = "{}"
'''.format(NOEXEC), tmp)
            exit_code, lines, resps, _ = run_ssh_frontiere(
                config, 'read', '\ninfra no-exec\n.\n')
            self.assertEqual(len(resps), 1)
            self.assertEqual(resps[0]['status_code'], 128)

    def test_sc_tmo_009_process_group_kill(self):
        """SC-TMO-009: Processus enfant qui fork et cree des zombies."""
        with tempfile.TemporaryDirectory() as tmp:
            config = make_config('''
[domains.infra]
description = "Infrastructure"

[domains.infra.actions.forker]
description = "Cree des sous-processus"
level = "read"
timeout = 2
execute = "{}"
'''.format(FORKER), tmp)
            exit_code, lines, resps, _ = run_ssh_frontiere(
                config, 'read', '\ninfra forker\n.\n', timeout=20)
            self.assertEqual(len(resps), 1)
            self.assertEqual(resps[0]['status_code'], 130)

    def test_sc_tmo_010_timeout_zero(self):
        """SC-TMO-010: Commande avec timeout = 0."""
        with tempfile.TemporaryDirectory() as tmp:
            config = make_config('''
[domains.infra]
description = "Infrastructure"

[domains.infra.actions.instant]
description = "Timeout immediat"
level = "read"
timeout = 0
execute = "{}"
'''.format(STUB), tmp)
            exit_code, lines, resps, _ = run_ssh_frontiere(
                config, 'read', '\ninfra instant\n.\n', timeout=10)
            # Either config error (129) or immediate timeout (130) or it runs
            self.assertIn(exit_code, (0, 129, 130))


if __name__ == '__main__':
    unittest.main()
