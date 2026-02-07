# Build stage
FROM golang:1.21-alpine AS builder

RUN apk add --no-cache gcc musl-dev sqlite-dev

WORKDIR /build

COPY go.mod go.sum ./
RUN go mod download

COPY . .

RUN CGO_ENABLED=1 go build -ldflags "-s -w -X main.version=0.1.0" -o bd-workflow ./cmd/bd-workflow/main.go

# Runtime stage
FROM alpine:3.19

RUN apk add --no-cache sqlite-libs ca-certificates

RUN adduser -D -h /app workflow
USER workflow
WORKDIR /app

COPY --from=builder /build/bd-workflow /app/bd-workflow
COPY --from=builder /build/migrations /app/migrations
COPY --from=builder /build/config/config.yaml /app/config/config.yaml

RUN mkdir -p /app/data /app/.beads

EXPOSE 8080

VOLUME ["/app/data", "/app/.beads"]

ENTRYPOINT ["/app/bd-workflow"]
CMD ["status"]