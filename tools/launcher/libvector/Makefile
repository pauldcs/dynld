NAME          := libvector.a
CC            := clang
SRCS_DIR      := srcs
OBJS_DIR      := .objs
BUILD_DIR     := build
INCS_DIR      := incs

CFLAGS := \
	-fstack-protector        \
	-fstack-protector-strong \
	-fstack-check            \
	-O3

CFLAGS_DBG := \
-g3                         \
-Og                         \
-Wall                       \
-Werror                     \
-Wextra                     \
-pedantic                   \
-fsanitize=address          \
-fsanitize=undefined        \
-fno-omit-frame-pointer     \
-fno-optimize-sibling-calls \
-D DEBUG=1

SRCS := \
	vector.c

SRCS_OBJS := $(patsubst %.c,$(OBJS_DIR)/%.o,$(SRCS))

$(OBJS_DIR)/%.o:$(SRCS_DIR)/%.c
	@mkdir -vp $(dir $@)
	$(CC) \
		$(CFLAGS) \
		-MMD \
		-MP \
		-o $@ \
		-c $< \
		-I $(INCS_DIR)

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

.PHONY	: all clean g fclean re

