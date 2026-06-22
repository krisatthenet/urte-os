# URTE OS core - build file
# POSIX-conformant build; standard C11 + _POSIX_C_SOURCE feature test macro.

CC      ?= gcc
CFLAGS  ?= -std=c11 -D_POSIX_C_SOURCE=200809L -Wall -Wextra -Iinclude
# POSIX message queues need -lrt (glibc); the channel registry lock needs pthread
LDFLAGS ?= -lrt -lpthread

OBJ = kernel/urte_core.o
BIN = urte_demo

.PHONY: all clean test

all: $(BIN)

$(BIN): $(OBJ) kernel/urte_demo.o
	$(CC) $(CFLAGS) -o $@ $^ $(LDFLAGS)

%.o: %.c
	$(CC) $(CFLAGS) -c -o $@ $<

test: $(BIN)
	./$(BIN)

clean:
	rm -f $(OBJ) kernel/urte_demo.o $(BIN)
