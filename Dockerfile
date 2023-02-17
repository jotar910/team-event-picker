# use the main official rust docker image
FROM rust

# copy the app into the docker image
COPY . /app

# set the work directory
WORKDIR /app

# build the app
RUN cargo build --release

# start the app
CMD [ "./target/release/team-event-picker" ]
