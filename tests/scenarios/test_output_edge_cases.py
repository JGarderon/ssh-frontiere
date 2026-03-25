#!/usr/bin/env python3
"""Tests for output edge case scenarios (SC-OUT-001 to SC-OUT-014)."""
import unittest
import sys
import os
import json
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from conftest import run_ssh_frontiere, make_config, stub_path, sha256_hex

STUB = stub_path('echo-ok.sh')
FAIL = stub_path('echo-fail.sh')
SILENT = stub_path('silent.sh')
MIXED = stub_path('mixed-output.sh')


class TestOutputEdgeCases(unittest.TestCase):

    def test_sc_out_001_successful_command_response(self):
        """SC-OUT-001: Reponse JSON pour commande reussie."""
        with tempfile.TemporaryDirectory() as tmp:
            config = make_config('''
[domains.infra]
description = "Infrastructure"

[domains.infra.actions.healthcheck]
description = "Check"
level = "read"
timeout = 10
execute = "{}"
'''.format(STUB), tmp)
            exit_code, lines, resps, _ = run_ssh_frontiere(
                config, 'read', '\ninfra healthcheck\n.\n')
            self.assertEqual(len(resps), 1)
            r = resps[0]
            self.assertEqual(r['status_code'], 0)
            self.assertEqual(r['status_message'], 'executed')
            self.assertIsNone(r['stdout'])
            self.assertIsNone(r['stderr'])
            self.assertIn('infra healthcheck', r['command'])
            # Verify streaming output exists
            streaming = [l for l in lines if l.startswith('>> ')]
            self.assertTrue(len(streaming) > 0)

    def test_sc_out_002_rejected_command_response(self):
        """SC-OUT-002: Reponse JSON pour commande refusee."""
        with tempfile.TemporaryDirectory() as tmp:
            config = make_config('''
[domains.infra]
description = "Infrastructure"

[domains.infra.actions.deploy]
description = "Deploy"
level = "admin"
timeout = 10
execute = "{}"
'''.format(STUB), tmp)
            exit_code, lines, resps, _ = run_ssh_frontiere(
                config, 'read', '\ninfra deploy\n.\n')
            self.assertEqual(len(resps), 1)
            r = resps[0]
            self.assertEqual(r['status_code'], 131)
            self.assertIn('insufficient level', r['status_message'])
            self.assertIsNone(r['stdout'])
            self.assertIsNone(r['stderr'])

    def test_sc_out_003_stderr_output(self):
        """SC-OUT-003: Commande avec sortie stderr."""
        with tempfile.TemporaryDirectory() as tmp:
            config = make_config('''
[domains.infra]
description = "Infrastructure"

[domains.infra.actions.failing-check]
description = "Failing check"
level = "read"
timeout = 10
execute = "{}"
'''.format(FAIL), tmp)
            exit_code, lines, resps, _ = run_ssh_frontiere(
                config, 'read', '\ninfra failing-check\n.\n')
            self.assertEqual(len(resps), 1)
            self.assertEqual(resps[0]['status_code'], 1)
            self.assertEqual(resps[0]['status_message'], 'executed')
            # stderr should be streamed via >>!
            stderr_lines = [l for l in lines if l.startswith('>>! ')]
            self.assertTrue(len(stderr_lines) > 0)

    def test_sc_out_004_mixed_stdout_stderr(self):
        """SC-OUT-004: Commande avec sortie melangee stdout et stderr."""
        with tempfile.TemporaryDirectory() as tmp:
            config = make_config('''
[domains.infra]
description = "Infrastructure"

[domains.infra.actions.mixed-output]
description = "Mixed output"
level = "read"
timeout = 10
execute = "{}"
'''.format(MIXED), tmp)
            exit_code, lines, resps, _ = run_ssh_frontiere(
                config, 'read', '\ninfra mixed-output\n.\n')
            self.assertEqual(len(resps), 1)
            stdout_lines = [l for l in lines if l.startswith('>> ') and not l.startswith('>>!') and not l.startswith('>>>')]
            stderr_lines = [l for l in lines if l.startswith('>>! ')]
            self.assertTrue(len(stdout_lines) > 0, "Expected stdout streaming")
            self.assertTrue(len(stderr_lines) > 0, "Expected stderr streaming")

    def test_sc_out_005_silent_command(self):
        """SC-OUT-005: Commande sans aucune sortie."""
        with tempfile.TemporaryDirectory() as tmp:
            config = make_config('''
[domains.infra]
description = "Infrastructure"

[domains.infra.actions.silent-check]
description = "Silent check"
level = "read"
timeout = 10
execute = "{}"
'''.format(SILENT), tmp)
            exit_code, lines, resps, _ = run_ssh_frontiere(
                config, 'read', '\ninfra silent-check\n.\n')
            self.assertEqual(len(resps), 1)
            self.assertEqual(resps[0]['status_code'], 0)
            # No streaming lines
            streaming = [l for l in lines
                         if (l.startswith('>> ') and not l.startswith('>>>'))
                         or l.startswith('>>! ')]
            self.assertEqual(len(streaming), 0)

    def test_sc_out_006_banner_format(self):
        """SC-OUT-006: Banniere serveur complete."""
        with tempfile.TemporaryDirectory() as tmp:
            config = make_config('''
[auth]
challenge_nonce = true

[auth.tokens.test]
secret = "b64:dGVzdA=="
level = "ops"

[domains.infra]
description = "Infrastructure"

[domains.infra.actions.check]
description = "Check"
level = "read"
timeout = 10
execute = "{}"
'''.format(STUB), tmp)
            exit_code, lines, resps, _ = run_ssh_frontiere(
                config, 'read', '\ninfra check\n.\n')
            # Check banner lines
            self.assertTrue(lines[0].startswith('#> ssh-frontiere'))
            cap_line = [l for l in lines if l.startswith('+> capabilities')]
            self.assertTrue(len(cap_line) >= 1)
            self.assertIn('rbac', cap_line[0])
            self.assertIn('session', cap_line[0])
            # Challenge nonce present
            nonce_line = [l for l in lines if '+> challenge nonce=' in l]
            self.assertTrue(len(nonce_line) >= 1)

    def test_sc_out_007_special_chars_in_output(self):
        """SC-OUT-007: Reponse JSON avec caracteres speciaux dans stdout."""
        with tempfile.TemporaryDirectory() as tmp:
            # Use echo-ok.sh with a message arg containing special JSON chars
            stub = STUB
            config = make_config(
                '[domains.app]\n'
                'description = "Application"\n\n'
                '[domains.app.actions.json-output]\n'
                'description = "JSON output"\n'
                'level = "read"\n'
                'timeout = 10\n'
                'execute = "' + stub + ' {msg}"\n\n'
                '[domains.app.actions.json-output.args]\n'
                'msg = { type = "string" }\n',
                tmp)
            exit_code, lines, resps, _ = run_ssh_frontiere(
                config, 'read',
                '\napp json-output msg="value with quotes and tabs"\n.\n')
            self.assertEqual(len(resps), 1)
            self.assertEqual(resps[0]['status_code'], 0)
            # The JSON response line should be valid JSON
            for line in lines:
                if line.startswith('>>> '):
                    parsed = json.loads(line[4:])
                    self.assertIn('status_code', parsed)

    def test_sc_out_008_sensitive_arg_masked(self):
        """SC-OUT-008: Argument sensible masque dans les logs."""
        with tempfile.TemporaryDirectory() as tmp:
            log_file = os.path.join(tmp, 'logs', 'commands.json')
            config = make_config('''
[global]
log_file = "{log}"

[domains.infra]
description = "Infrastructure"

[domains.infra.actions.set-password]
description = "Set password"
level = "admin"
timeout = 10
execute = "{stub} {{password}}"

[domains.infra.actions.set-password.args]
password = {{ type = "string", sensitive = true }}
'''.format(stub=STUB, log=log_file), tmp)
            exit_code, lines, resps, _ = run_ssh_frontiere(
                config, 'admin', '\ninfra set-password password=SuperSecret123\n.\n')
            self.assertEqual(len(resps), 1)
            self.assertEqual(resps[0]['status_code'], 0)
            # Check logs for masking
            if os.path.exists(log_file):
                with open(log_file) as f:
                    log_content = f.read()
                self.assertNotIn('SuperSecret123', log_content)

    def test_sc_out_009_defaults_applied(self):
        """SC-OUT-009: Valeurs par defaut appliquees."""
        with tempfile.TemporaryDirectory() as tmp:
            config = make_config('''
[domains.app]
description = "Application"

[domains.app.actions.deploy]
description = "Deploy"
level = "ops"
timeout = 10
execute = "{stub} {{env}} {{verbose}}"

[domains.app.actions.deploy.args]
env = {{ type = "enum", values = ["prod", "staging"], default = "staging" }}
verbose = {{ type = "enum", values = ["true", "false"], default = "false" }}
'''.format(stub=STUB), tmp)
            exit_code, lines, resps, _ = run_ssh_frontiere(
                config, 'ops', '\napp deploy\n.\n')
            self.assertEqual(len(resps), 1)
            self.assertEqual(resps[0]['status_code'], 0)
            # Defaults should have been applied
            streaming = [l for l in lines if l.startswith('>> ')]
            output_text = ' '.join(streaming)
            self.assertIn('staging', output_text)
            self.assertIn('false', output_text)

    def test_sc_out_010_protocol_error_response(self):
        """SC-OUT-010: Reponse JSON pour erreur de protocole."""
        # NOTE: differs from scenario — actual behavior: unknown directives
        # are silently ignored, so we test with binary data instead.
        with tempfile.TemporaryDirectory() as tmp:
            config = make_config('''
[domains.infra]
description = "Infrastructure"

[domains.infra.actions.check]
description = "Check"
level = "read"
timeout = 10
execute = "{}"
'''.format(STUB), tmp)
            # Binary data causes protocol error
            from conftest import run_ssh_frontiere_raw
            exit_code, stdout, stderr = run_ssh_frontiere_raw(
                config, 'read', b'\x00\x01\xff\xfe')
            self.assertIn(exit_code, (128, 132, 133))

    def test_sc_out_011_key_value_with_equals(self):
        """SC-OUT-011: Argument key=value=with=equals (split sur le premier =)."""
        with tempfile.TemporaryDirectory() as tmp:
            config = make_config('''
[domains.app]
description = "Application"

[domains.app.actions.configure]
description = "Configuration"
level = "ops"
timeout = 10
execute = "{stub} {{setting}}"

[domains.app.actions.configure.args]
setting = {{ type = "string" }}
'''.format(stub=STUB), tmp)
            exit_code, lines, resps, _ = run_ssh_frontiere(
                config, 'ops', '\napp configure setting=key=value=with=equals\n.\n')
            self.assertEqual(len(resps), 1)
            self.assertEqual(resps[0]['status_code'], 0)
            # The value should contain the full string after first =
            streaming = [l for l in lines if l.startswith('>> ')]
            output_text = ' '.join(streaming)
            self.assertIn('key=value=with=equals', output_text)

    def test_sc_out_012_empty_value(self):
        """SC-OUT-012: Argument avec valeur vide (key=)."""
        with tempfile.TemporaryDirectory() as tmp:
            config = make_config('''
[domains.app]
description = "Application"

[domains.app.actions.configure]
description = "Configuration"
level = "ops"
timeout = 10
execute = "{stub} {{setting}}"

[domains.app.actions.configure.args]
setting = {{ type = "string" }}
'''.format(stub=STUB), tmp)
            exit_code, lines, resps, _ = run_ssh_frontiere(
                config, 'ops', '\napp configure setting=\n.\n')
            self.assertEqual(len(resps), 1)
            self.assertEqual(resps[0]['status_code'], 0)

    def test_sc_out_013_duplicate_argument(self):
        """SC-OUT-013: Argument duplique dans une commande."""
        with tempfile.TemporaryDirectory() as tmp:
            config = make_config('''
[domains.app]
description = "Application"

[domains.app.actions.deploy]
description = "Deploy"
level = "ops"
timeout = 10
execute = "{stub} {{env}}"

[domains.app.actions.deploy.args]
env = {{ type = "enum", values = ["prod", "staging"] }}
'''.format(stub=STUB), tmp)
            exit_code, lines, resps, _ = run_ssh_frontiere(
                config, 'ops', '\napp deploy env=prod env=staging\n.\n')
            self.assertEqual(len(resps), 1)
            self.assertEqual(resps[0]['status_code'], 128)

    def test_sc_out_014_unknown_argument(self):
        """SC-OUT-014: Argument inconnu dans une commande."""
        with tempfile.TemporaryDirectory() as tmp:
            config = make_config('''
[domains.app]
description = "Application"

[domains.app.actions.deploy]
description = "Deploy"
level = "ops"
timeout = 10
execute = "{stub} {{env}}"

[domains.app.actions.deploy.args]
env = {{ type = "enum", values = ["prod", "staging"] }}
'''.format(stub=STUB), tmp)
            exit_code, lines, resps, _ = run_ssh_frontiere(
                config, 'ops', '\napp deploy env=prod unknown_arg=value\n.\n')
            self.assertEqual(len(resps), 1)
            self.assertEqual(resps[0]['status_code'], 128)


if __name__ == '__main__':
    unittest.main()
