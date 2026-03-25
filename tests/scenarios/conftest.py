import subprocess
import os
import json
import hashlib
import tempfile

BINARY = os.path.join(os.path.dirname(os.path.abspath(__file__)), '../../target/debug/ssh-frontiere')
STUBS_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'stubs')
CONFIGS_DIR = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'configs')


def run_ssh_frontiere(config_path, level, stdin_text, timeout=10, env_extra=None):
    """Launch the binary and return (exit_code, stdout_lines, response_json_list, stderr_text)."""
    env = {'PATH': '/usr/local/bin:/usr/bin:/bin'}
    if env_extra:
        env.update(env_extra)
    proc = subprocess.run(
        [BINARY, '--level={}'.format(level), '--config={}'.format(config_path)],
        input=stdin_text.encode('utf-8', errors='replace') if isinstance(stdin_text, str) else stdin_text,
        capture_output=True,
        timeout=timeout,
        env=env,
    )
    exit_code = proc.returncode
    stdout = proc.stdout.decode('utf-8', errors='replace')
    stderr = proc.stderr.decode('utf-8', errors='replace')
    stdout_lines = stdout.splitlines()

    responses = []
    for line in stdout_lines:
        if line.startswith('>>> '):
            try:
                responses.append(json.loads(line[4:]))
            except json.JSONDecodeError:
                pass

    return exit_code, stdout_lines, responses, stderr


def run_ssh_frontiere_raw(config_path, level, stdin_bytes, timeout=10):
    """Launch binary with raw bytes input."""
    env = {'PATH': '/usr/local/bin:/usr/bin:/bin'}
    proc = subprocess.run(
        [BINARY, '--level={}'.format(level), '--config={}'.format(config_path)],
        input=stdin_bytes,
        capture_output=True,
        timeout=timeout,
        env=env,
    )
    return proc.returncode, proc.stdout, proc.stderr


def make_config(toml_content, temp_dir):
    """Write a TOML config into a temp file and return the path."""
    if isinstance(temp_dir, str):
        config_path = os.path.join(temp_dir, 'config.toml')
        log_dir = os.path.join(temp_dir, 'logs')
    else:
        config_path = os.path.join(str(temp_dir), 'config.toml')
        log_dir = os.path.join(str(temp_dir), 'logs')
    os.makedirs(log_dir, exist_ok=True)

    if 'log_file' not in toml_content:
        log_file = os.path.join(log_dir, 'commands.json')
        log_line = 'log_file = "{}"\n'.format(log_file)
        if '[global]' in toml_content:
            toml_content = toml_content.replace(
                '[global]\n', '[global]\n' + log_line, 1)
        else:
            toml_content = '[global]\n' + log_line + toml_content

    with open(config_path, 'w') as f:
        f.write(toml_content)
    return config_path


def sha256_hex(text):
    """Return SHA-256 hex digest of text."""
    return hashlib.sha256(text.encode()).hexdigest()


def stub_path(name):
    """Return absolute path to a stub script."""
    return os.path.join(STUBS_DIR, name)
