VERSION := 2.0.2
.DEFAULT_GOAL := rulmee

CDIR = src
LDIR = lib
IDIR = include
ODIR = dist

PREFIX = /usr/local

INFO_GIT_REV ?= $(shell git describe --long --tags --always || echo '?')
INFO_GIT_REV := $(INFO_GIT_REV)
INFO_BUILD_TS ?= $(shell date +%s)
INFO_BUILD_TS := $(INFO_BUILD_TS)

CFLAGS ?= -O3 -Wall -Wextra -fdata-sections -ffunction-sections
# C PreProcessor flags, not C Plus Plus
CPPFLAGS ?=
_DFLAGS?= \
	-DRULMEE_VERSION=\"$(VERSION)\" \
	-DRULMEE_GIT_REV=\"$(INFO_GIT_REV)\" \
	-DRULMEE_BUILD_TS=$(INFO_BUILD_TS)
ALLFLAGS = $(_DFLAGS) $(CFLAGS) $(CPPFLAGS) -I$(IDIR)
LDFLAGS ?= -Wl,--gc-sections

LIBS = -lpam

# includes all headers in `$(IDIR)` and compiles everything in `$(CDIR)`
DEPS = $(wildcard $(IDIR)/*.h $(IDIR)/**/*.h)
_SOURCES = $(wildcard $(CDIR)/*.c) $(wildcard $(CDIR)/**/*.c)
OBJ = $(patsubst $(CDIR)/%.c,$(ODIR)/%.o,$(_SOURCES))

$(ODIR)/%.o: $(CDIR)/%.c $(DEPS)
	@mkdir -p $(dir $@)
	$(CC) -c -o $@ $< $(ALLFLAGS)

rulmee: $(OBJ)
	$(CC) -o $@ $^ $(ALLFLAGS) $(LIBS) $(LDFLAGS)

clean:
	rm -rf $(ODIR) rulmee

install: rulmee
	mkdir -p ${DESTDIR}${PREFIX}/bin ${DESTDIR}${PREFIX}/share/man/man{1,5}
	install -Dm755 ./rulmee ${DESTDIR}${PREFIX}/bin/
	[ -f ${DESTDIR}/etc/rulmee/default.toml ] || install -Dm644 ./themes/default.toml ${DESTDIR}/etc/rulmee/default.toml
	install -Dm644 ./assets/man/rulmee.1 ${DESTDIR}${PREFIX}/share/man/man1/
	install -Dm644 ./assets/man/rulmee-config.5 ${DESTDIR}${PREFIX}/share/man/man5/

uninstall: uninstall-service
	rm -rf ${DESTDIR}${PREFIX}/bin/rulmee ${DESTDIR}/etc/rulmee/default.toml
	rm -rf ${DESTDIR}${PREFIX}/share/man/man{1/rulmee.1,5/rulmee-config.5}.gz

include services.mk

pre-commit:
	codespell
	prettier -c "**/*.md"
	git ls-files "*.sh" "*/PKGBUILD" | xargs shellcheck --shell=bash
	clang-format -i $$(git ls-files "*.c" "*.h")
	git ls-files -z "*.c" "*.h" | \
		parallel -j$$(nproc) -q0 --no-notice --will-cite --tty clang-tidy -warnings-as-errors=\* --quiet |& \
		grep -v "warnings generated." || true

print-version:
	@echo $(VERSION)

.PHONY: clean \
	install uninstall \
	pre-commit \
	print-version
