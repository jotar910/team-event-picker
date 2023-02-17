# use the main official rust docker image as our builder
FROM rust as builder

# copy the app into the docker image
COPY . /app

# set the work directory
WORKDIR /app

# build the app
RUN cargo build --release

# use google distroless as runtime image
FROM gcr.io/distroless/cc-debian11

# copy the app from builder image to this runtime image
COPY --from=builder /app/target/release/team-event-picker /app/team-event-picker

# set the work directory
WORKDIR /app

# start the app
CMD [ "./team-event-picker" ]
