NAME       := libtlv.a
CC         := clang
SRCS_DIR   := .
OBJS_DIR   := .objs
BUILD_DIR  := build
INCS_DIR   := incs
CFLAGS     := \
	-Wall \
	-Wextra \
	-Werror \
	-pedantic \
	-U_FORTIFY_SOURCE \
	-D_FORTIFY_SOURCE=0 \
	-mmacosx-version-min=12.0

CFLAGS_DBG := \
	-g3                      \
	-O0                      \
	-fsanitize=address       \
	-fsanitize=undefined     \
	-fno-omit-frame-pointer  \
	-fstack-protector-strong \
	-fno-optimize-sibling-calls

SRCS := \
	thread_local_variables.c \
	thread_local_helpers.s

SRCS_OBJS := $(patsubst %.c,$(OBJS_DIR)/%.o,$(filter %.c,$(SRCS))) \
             $(patsubst %.s,$(OBJS_DIR)/%.o,$(filter %.s,$(SRCS)))

$(OBJS_DIR)/%.o:$(SRCS_DIR)/%.c
	@mkdir -vp $(dir $@)
	$(CC) \
		$(CFLAGS) \
		-MMD \
		-MP \
		-o $@ \
		-c $< \
		-I $(INCS_DIR)

$(OBJS_DIR)/%.o:$(SRCS_DIR)/%.s
	@mkdir -vp $(dir $@)
	$(CC) \
		$(CFLAGS) \
		-o $@ \
		-c $<

all: $(NAME)

-include  $(SRCS_OBJS:.o=.d)

$(NAME): $(SRCS_OBJS)
	ar rc \
		$(NAME) \
		$(SRCS_OBJS)

g: CFLAGS += $(CFLAGS_DBG)
g: all

clean:
	rm -rf $(OBJS_DIR)
	rm -rf .cache
	rm -rf .dSYM

fclean: clean
	rm -rf $(NAME)

re: fclean all

.PHONY  : all clean g fclean re