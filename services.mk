install-service:
	@if command -v systemctl &> /dev/null; then \
		make install-service-systemd; \
	elif command -v dinitctl &> /dev/null; then \
		make install-service-dinit; \
	elif command -v sv &> /dev/null; then \
		if [ -d /etc/sv ]; then \
			make install-service-runit; \
		elif [ -d /etc/runit/sv ]; then \
			make install-service-runit-etc; \
		else \
			printf '\033[31m%s\033[0m\n' "Unknown init system structure, skipping service install..." >&2; \
		fi \
	elif command -v rc-update &> /dev/null; then \
		make install-service-openrc; \
	elif command -v s6-service &> /dev/null; then \
		if [ -d /etc/sv ]; then \
			make install-service-s6; \
		elif [ -d /etc/r6nit/sv ]; then \
			make install-service-s6-etc; \
		else \
			printf '\033[31m%s\033[0m\n' "Unknown init system structure, skipping service install..." >&2; \
		fi \
	else \
		printf '\033[1;31m%s\033[0m\n' "Unknown init system, skipping service install..." >&2; \
	fi

install-service-systemd:
	@sed -e 's|ExecStart=/usr/bin/rulmee|ExecStart=${DESTDIR}${PREFIX}/bin/rulmee|' ./assets/services/systemd.service > ./dist/rulmee.service
	install -Dm644 ./dist/rulmee.service ${DESTDIR}${PREFIX}/lib/systemd/system/rulmee.service
	@printf '\033[1m%s\033[0m\n\n' " don't forget to run 'systemctl enable rulmee'"
install-service-dinit:
	install -m644 ./assets/services/dinit ${DESTDIR}/etc/dinit.d/rulmee
	@printf '\033[1m%s\033[0m\n\n' " don't forget to run 'dinitctl enable rulmee'"
install-service-runit:
	@if [ ! -e /etc/sv ] && [ -d /etc/runit/sv ] && [ -z "$$FORCE" ]; then \
		printf '\033[31m%s\033[0m\n' "/etc/sv doesn't exist but /etc/runit/sv does" >&2; \
		printf '\033[31m%s\033[0m\n' "you probably meant to 'make install-service-runit-etc'" >&2; \
		exit 1; \
	fi
	mkdir -p ${DESTDIR}/etc/sv/rulmee
	cp -r --update=all ./assets/services/runit/* ${DESTDIR}/etc/sv/rulmee/
	@printf '\033[1m%s\033[0m\n\n' " don't forget to run 'ln -s ${DESTDIR}/etc/sv/rulmee /var/service' or your distro equivalent"
install-service-runit-etc:
	@if [ ! -e /etc/runit/sv ] && [ -d /etc/sv ] && [ -z "$$FORCE" ]; then \
		printf '\033[31m%s\033[0m\n' "/etc/runit/sv doesn't exist but /etc/sv does" >&2; \
		printf '\033[31m%s\033[0m\n' "you probably meant to 'make install-service-runit'" >&2; \
		exit 1; \
	fi
	mkdir -p ${DESTDIR}/etc/runit/sv/rulmee
	cp -r --update=all ./assets/services/runit/* ${DESTDIR}/etc/runit/sv/rulmee/
	@printf '\033[1m%s\033[0m\n\n' " don't forget to run 'ln -s ${DESTDIR}/etc/runit/sv/rulmee /run/runit/service' or your distro equivalent"
install-service-openrc:
	install -m755 ./assets/services/openrc ${DESTDIR}/etc/init.d/rulmee
	@printf '\033[1m%s\033[0m\n\n' " don't forget to run 'rc-update add rulmee'"
install-service-s6:
	@if [ ! -e /etc/sv ] && [ -d /etc/s6/sv ] && [ -z "$$FORCE" ]; then \
		printf '\033[31m%s\033[0m\n' "/etc/sv doesn't exist but /etc/s6/sv does" >&2; \
		printf '\033[31m%s\033[0m\n' "you probably meant to 'make install-service-s6-etc'" >&2; \
		exit 1; \
	fi
	mkdir -p ${DESTDIR}/etc/sv/rulmee
	cp -r --update=all ./assets/services/s6/* ${DESTDIR}/etc/sv/rulmee/
	@printf '\033[1m%s\033[0m\n\n' " don't forget to run 's6-service add default rulmee' and 's6-db-reload'"
install-service-s6-etc:
	@if [ ! -e /etc/s6/sv ] && [ -d /etc/sv ] && [ -z "$$FORCE" ]; then \
		printf '\033[31m%s\033[0m\n' "/etc/s6/sv doesn't exist but /etc/sv does" >&2; \
		printf '\033[31m%s\033[0m\n' "you probably meant to 'make install-service-s6'" >&2; \
		exit 1; \
	fi
	mkdir -p ${DESTDIR}/etc/s6/sv/rulmee
	cp -r --update=all ./assets/services/s6/* ${DESTDIR}/etc/s6/sv/rulmee/
	@printf '\033[1m%s\033[0m\n\n' " don't forget to run 's6-service add default rulmee' and 's6-db-reload'"

uninstall-service:
	rm -rf \
		${DESTDIR}${PREFIX}/lib/systemd/system/rulmee.service \
		${DESTDIR}/etc/dinit.d/rulmee \
		${DESTDIR}/etc/sv/rulmee \
		${DESTDIR}/etc/runit/sv/rulmee \
		${DESTDIR}/etc/init.d/rulmee \
		${DESTDIR}/etc/s6/sv/rulmee

.PHONY: install-service uninstall-service \
	install-service-s6 \
	install-service-s6-etc \
	install-service-dinit \
	install-service-runit \
	install-service-runit-etc \
	install-service-openrc \
	install-service-systemd
