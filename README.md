nanocron

A tiny cron-like daemon, which can only run one schedule and only one command.


## Why was it created?

In my Kubernetes cluster, I use sidecar containers for backups, which is necessary in StatefulSets to directly access the volumes. This tool allows me to schedule jobs to run at a certain time, without have to use weird(er hacks.)


## Usage

The [container available at the Docker Hub](@todo) is kinda useless by itself, it's meant to be used as a base as part of a multi-stage-build:

```Dockerfile
FROM dermitch/nanocron AS nanocron

FROM ubuntu:20.04

COPY --from=nanocron /usr/bin/nanocron /usr/bin/nanocron

# Add whatever you need

# Run each day at midnight (UTC only supported atm)
CMD ["/usr/bin/nanocron", "0 * * * *", "/run_backup.sh"]
```

