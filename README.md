# system-info

A Yoke Plugin that tells what host it runs on.

- It answers the question `system.info` with a snapshot of the host: hostname, OS, kernel, CPU, cores,
  memory, uptime, and under `os_release` what the operating system says of itself in `os-release`: its
  `id`, `name`, `pretty_name`, `logo`, and its `home_url`, `documentation_url`, `support_url`,
  `bug_report_url` and `privacy_policy_url`, each absent where the file has none.
- It publishes the stream `system.uptime`, one frame a second while the stream flows.

Nothing is granted by declaring it: an operator grants `query.info.answer` and `stream.uptime.publish`.

Take it into an application with `yoke-compose add plugin https://github.com/yoke-project/yoke-plugin-system-info@<tag>`.
Build it with `yoke-compose build`, and try it from `try/`.

Licensed under the Apache License 2.0.
