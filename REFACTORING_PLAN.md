# Issue

logs: &mut EventRecorder, // there should be a distinction between logs array and event recorder. The logs appears as vector while it isn't

# Biggest Todo: record issues in the client automatically to sentry or a similar service (or our own service obviously)

# I might have found an issue in the batch send function because there are 2 times logs in the object and it might not get sent properly

Request body:
{"logs":[{"logs":[{"attributes":null,"event_status":"test_event","event_type":"process_status","message":"Test event","process_type":"pipeline","timestamp":1719527406},{"attributes":{"available_memory":0,"cpu_usage_percentage":0.0,"disk_data":[{"Macintosh HD":{"disk_available_space":20527308022,"disk_total_space":245107195904,"disk_used_space":224579887882,"disk_utilization":91.6251712046676}},{"Macintosh HD":{"disk_available_space":20527308022,"disk_total_space":245107195904,"disk_used_space":224579887882,"disk_utilization":91.6251712046676}}],"events_name":"global_system_metrics","memory_utilization":null,"total_memory":0,"used_memory":0},"event_status":"metric_event","event_type":"process_status","message":"[2024-06-27 22:30:06.527128 UTC] System's resources metric","process_type":"pipeline","timestamp":1719527406}]}]}

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
