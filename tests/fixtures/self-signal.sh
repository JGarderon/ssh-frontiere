#!/bin/sh
# Script that sends itself SIGTERM — exits with signal 15
# Used by chain_exec_tests to cover the Signaled branch
kill -TERM $$
