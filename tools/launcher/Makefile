NAME            := run
CC              := gcc
SRCS_DIR        := srcs
OBJS_DIR        := .objs
INCS_DIR        := incs
DYNLD_DIR       := ../..
LIBTLV_DIR      := ../../libtlv
LIBDSC_DIR      := ../../libdsc

DEBUG ?= 0
ifeq ($(DEBUG), 1)
    BUILD_MODE := debug
    CARGO_FLAGS :=
    CFLAGS_MODE := \
			-D DEBUG                    \
			-Og                         \
			-Wall                       \
			-Werror                     \
			-Wextra                     \
			-pedantic                   \
			-fsanitize=address          \
			-fsanitize=undefined        \
			-fno-omit-frame-pointer     \
			-fno-optimize-sibling-calls
else
    BUILD_MODE := release
    CARGO_FLAGS := --release
    CFLAGS_MODE := -O3
endif

CFLAGS := \
    $(CFLAGS_MODE)

SRCS := \
    main.c

SRCS_OBJS := $(patsubst %.c,$(OBJS_DIR)/%.o,$(SRCS))

$(OBJS_DIR)/%.o:$(SRCS_DIR)/%.c
	@mkdir -vp $(dir $@)
	$(CC) \
	    $(CFLAGS) \
	    -MMD \
	    -MP \
	    -o $@ \
	    -c $< \
	    -fPIC \
	    -I $(INCS_DIR)

.DEFAULT_GOAL := all

all: $(NAME)

g:
	$(MAKE) DEBUG=1

-include $(SRCS_OBJS:.o=.d)

dynld:
	cargo build --manifest-path $(DYNLD_DIR)/Cargo.toml $(CARGO_FLAGS)

$(NAME): $(SRCS_OBJS) dynld
	$(MAKE) -C $(LIBTLV_DIR)
	$(MAKE) -C $(LIBDSC_DIR)
	$(CC) $(filter-out dynld,$^) $(CFLAGS) \
	    -I $(INCS_DIR) \
			-Wl,-stack_size,0x10000000 \
			-L$(LIBTLV_DIR) -ltlv \
			-L$(LIBDSC_DIR) -ldsc \
	    -o $(NAME)

clean:
	$(MAKE) clean -C $(LIBTLV_DIR)
	$(MAKE) clean -C $(LIBDSC_DIR)
	rm -rf target
	rm -rf $(DYNLD_DIR)/target
	rm -rf build
	rm -rf *.dSYM
	rm -rf $(OBJS_DIR)

fclean: clean
	$(MAKE) fclean -C $(LIBTLV_DIR)
	$(MAKE) fclean -C $(LIBDSC_DIR)
	rm -rf $(NAME)

format:
	find . \( -name "*.c" -o -name "*.h" \) \
		-type f \
		-exec \
			clang-format \
			--verbose \
			-style=file -i {} \;

re: fclean all

.PHONY: all clean release fclean re g format