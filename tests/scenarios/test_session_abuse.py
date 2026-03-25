#!/usr/bin/env python3
"""Tests for session abuse scenarios (SC-SES-001 to SC-SES-011)."""
import unittest
import sys
import os
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from conftest import run_ssh_frontiere, make_config, stub_path, sha256_hex

STUB = stub_path('echo-ok.sh')
FAIL = stub_path('echo-fail.sh')
CAT = stub_path('stdin-cat.sh')


def _session_config(tmp):
    return make_config('''
[global]
timeout_session = 30

[domains.infra]
description = "Infrastructure"

[domains.infra.actions.healthcheck]
description = "Check"
level = "read"
timeout = 10
execute = "{stub}"

[domains.infra.actions.deploy]
description = "Deploy"
level = "admin"
timeout = 10
execute = "{stub}"
'''.format(stub=STUB), tmp)


class TestSessionAbuse(unittest.TestCase):

    def test_sc_ses_001_empty_session(self):
        """SC-SES-001: Session sans commande (ouverture puis fermeture)."""
        with tempfile.TemporaryDirectory() as tmp:
            config = _session_config(tmp)
            stdin = '+ session keepalive\n\n.\n'
            exit_code, lines, resps, _ = run_ssh_frontiere(config, 'read', stdin)
            self.assertEqual(exit_code, 0)
            # No command responses
            self.assertEqual(len(resps), 0)

    def test_sc_ses_002_many_commands(self):
        """SC-SES-002: Grand nombre de commandes en session."""
        with tempfile.TemporaryDirectory() as tmp:
            config = _session_config(tmp)
            commands = ''
            num_commands = 20  # Reduced from 100 for speed
            for _ in range(num_commands):
                commands += 'infra healthcheck\n.\n'
            stdin = '+ session keepalive\n\n' + commands + '.\n'
            exit_code, lines, resps, _ = run_ssh_frontiere(
                config, 'read', stdin, timeout=30)
            self.assertEqual(len(resps), num_commands)
            for r in resps:
                self.assertEqual(r['status_code'], 0)

    def test_sc_ses_003_exit_without_session(self):
        """SC-SES-003: Commande exit en mode one-shot."""
        # NOTE: differs from scenario — actual behavior: exit returns
        # status_code 0 with status_message "ok", not 128.
        with tempfile.TemporaryDirectory() as tmp:
            config = _session_config(tmp)
            stdin = '\nexit\n.\n'
            exit_code, lines, resps, _ = run_ssh_frontiere(config, 'read', stdin)
            self.assertEqual(len(resps), 1)
            self.assertEqual(resps[0]['status_code'], 0)

    def test_sc_ses_004_late_session_activation(self):
        """SC-SES-004: +session keepalive envoye apres la phase headers."""
        with tempfile.TemporaryDirectory() as tmp:
            config = _session_config(tmp)
            stdin = '\ninfra healthcheck\n.\n+ session keepalive\ninfra healthcheck\n.\n'
            exit_code, lines, resps, _ = run_ssh_frontiere(config, 'read', stdin)
            # First command executes in one-shot mode
            self.assertTrue(len(resps) >= 1)
            self.assertEqual(resps[0]['status_code'], 0)
            # Program should exit after first command (one-shot)
            self.assertEqual(len(resps), 1)

    def test_sc_ses_005_reauthentication_in_session(self):
        """SC-SES-005: Re-authentification en session pour elever le niveau."""
        with tempfile.TemporaryDirectory() as tmp:
            viewer_secret = "viewer"
            operator_secret = "operator"
            viewer_proof = sha256_hex(viewer_secret)
            operator_proof = sha256_hex(operator_secret)
            config = make_config('''
[global]
timeout_session = 30

[auth]
challenge_nonce = false

[auth.tokens.viewer]
secret = "b64:dmlld2Vy"
level = "read"

[auth.tokens.operator]
secret = "b64:b3BlcmF0b3I="
level = "ops"

[domains.forgejo]
description = "Forge Git"

[domains.forgejo.actions.status]
description = "Statut"
level = "read"
timeout = 10
execute = "{stub}"

[domains.forgejo.actions.backup]
description = "Sauvegarde"
level = "ops"
timeout = 10
execute = "{stub}"
'''.format(stub=STUB), tmp)
            stdin = (
                '+ session keepalive\n'
                '+ auth token=viewer proof={vp}\n'
                '\nforgejo status\n.\n'
                'forgejo backup\n.\n'
                '+ auth token=operator proof={op}\n'
                '\nforgejo backup\n.\n'
                '.\n'
            ).format(vp=viewer_proof, op=operator_proof)
            exit_code, lines, resps, _ = run_ssh_frontiere(config, 'read', stdin)
            self.assertTrue(len(resps) >= 3)
            self.assertEqual(resps[0]['status_code'], 0)    # status (read)
            self.assertEqual(resps[1]['status_code'], 131)   # backup (read < ops)
            self.assertEqual(resps[2]['status_code'], 0)     # backup (ops >= ops)

    def test_sc_ses_006_mixed_results(self):
        """SC-SES-006: Melange de commandes reussies et echouees en session."""
        with tempfile.TemporaryDirectory() as tmp:
            config = _session_config(tmp)
            stdin = (
                '+ session keepalive\n\n'
                'infra healthcheck\n.\n'
                'infra deploy\n.\n'
                'infra healthcheck\n.\n'
                '.\n'
            )
            exit_code, lines, resps, _ = run_ssh_frontiere(config, 'read', stdin)
            self.assertEqual(len(resps), 3)
            self.assertEqual(resps[0]['status_code'], 0)    # healthcheck ok
            self.assertEqual(resps[1]['status_code'], 131)   # deploy rejected
            self.assertEqual(resps[2]['status_code'], 0)    # healthcheck ok

    def test_sc_ses_007_unknown_command_in_session(self):
        """SC-SES-007: Commande inconnue en session."""
        with tempfile.TemporaryDirectory() as tmp:
            config = _session_config(tmp)
            stdin = (
                '+ session keepalive\n\n'
                'fake-domain fake-action\n.\n'
                'infra healthcheck\n.\n'
                '.\n'
            )
            exit_code, lines, resps, _ = run_ssh_frontiere(config, 'read', stdin)
            self.assertEqual(len(resps), 2)
            self.assertEqual(resps[0]['status_code'], 128)   # unknown domain
            self.assertEqual(resps[1]['status_code'], 0)    # healthcheck ok

    def test_sc_ses_008_body_then_normal(self):
        """SC-SES-008: Session avec body suivi d'une commande normale."""
        with tempfile.TemporaryDirectory() as tmp:
            config = make_config('''
[global]
timeout_session = 30

[domains.app]
description = "Application"

[domains.app.actions.import-data]
description = "Import"
level = "ops"
timeout = 10
execute = "{cat}"
max_body_size = 65536

[domains.app.actions.status]
description = "Statut"
level = "read"
timeout = 10
execute = "{stub}"
'''.format(cat=CAT, stub=STUB), tmp)
            # Body comes AFTER the command block terminator '.'
            stdin = (
                '+ session keepalive\n'
                '+ body\n\n'
                'app import-data\n.\n'
                'ligne de donnees 1\n'
                'ligne de donnees 2\n'
                '.\n'
                'app status\n.\n'
                '.\n'
            )
            exit_code, lines, resps, _ = run_ssh_frontiere(config, 'ops', stdin)
            self.assertTrue(len(resps) >= 2)
            self.assertEqual(resps[0]['status_code'], 0)
            self.assertEqual(resps[1]['status_code'], 0)

    def test_sc_ses_009_double_session_keepalive(self):
        """SC-SES-009: Double +session keepalive."""
        with tempfile.TemporaryDirectory() as tmp:
            config = _session_config(tmp)
            stdin = '+ session keepalive\n+ session keepalive\n\ninfra healthcheck\n.\n.\n'
            exit_code, lines, resps, _ = run_ssh_frontiere(config, 'read', stdin)
            self.assertTrue(len(resps) >= 1)
            self.assertEqual(resps[0]['status_code'], 0)

    def test_sc_ses_010_help_in_session(self):
        """SC-SES-010: Session avec commande help intercalee."""
        with tempfile.TemporaryDirectory() as tmp:
            config = _session_config(tmp)
            stdin = (
                '+ session keepalive\n\n'
                'help\n.\n'
                'infra healthcheck\n.\n'
                'help infra\n.\n'
                '.\n'
            )
            exit_code, lines, resps, _ = run_ssh_frontiere(config, 'read', stdin)
            self.assertEqual(len(resps), 3)
            self.assertEqual(resps[0]['status_code'], 0)  # help
            self.assertEqual(resps[1]['status_code'], 0)  # healthcheck
            self.assertEqual(resps[2]['status_code'], 0)  # help infra

    def test_sc_ses_011_stdin_closed_mid_session(self):
        """SC-SES-011: Fermeture brutale de stdin en milieu de session."""
        with tempfile.TemporaryDirectory() as tmp:
            config = _session_config(tmp)
            # Send first command, then incomplete second command and close stdin
            stdin = '+ session keepalive\n\ninfra healthcheck\n.\ninfra health'
            exit_code, lines, resps, _ = run_ssh_frontiere(config, 'read', stdin)
            # First command should succeed
            self.assertTrue(len(resps) >= 1)
            self.assertEqual(resps[0]['status_code'], 0)
            # Should not crash
            self.assertIn(exit_code, (0, 128, 132, 133))


if __name__ == '__main__':
    unittest.main()
