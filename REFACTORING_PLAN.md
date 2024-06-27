# Description of issue:

- No logs are appearing in the back_end url, thus we do not know if processes or metrics are being recorded.

# What is the source of the problem?

1. I don't know if submit_batched_data is working.
2. I don't know if polling process is collecting information.

# What is the fastest way to reproduce the problem?

# Structural improvement #1 Record all http_client events to a log file:

# Structural improvement #2 Record all polling processes to a log file so we will know if processes are being identified

# Structural improvement #3 Write a full integration test

- Atleast I will know the outgoing requests.

# Refactoring Rules

- Config may only be collected in main.rs or in tests

- Remove as much code as possible

- Switch back to pure functions and functional oriented programming.
