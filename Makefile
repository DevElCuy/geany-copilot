CARGO = /usr/bin/cargo-1.91
GEANY_CONFIG_DIR ?= $(HOME)/.config/geany
PLUGIN_DIR ?= $(GEANY_CONFIG_DIR)/plugins
PLUGIN_SOURCE = target/release/libcopilot_plugin.so
PLUGIN_NAME = copilot_plugin.so

.PHONY: build clean install uninstall

build:
	$(CARGO) build --release

clean:
	$(CARGO) clean

install: build
	mkdir -p $(PLUGIN_DIR)
	install -m 755 $(PLUGIN_SOURCE) $(PLUGIN_DIR)/$(PLUGIN_NAME)
	@echo "Plugin installed to $(PLUGIN_DIR)/$(PLUGIN_NAME)"
	@echo "Restart Geany, then enable it in Tools → Plugin Manager."

uninstall:
	rm -f $(PLUGIN_DIR)/$(PLUGIN_NAME)
	@echo "Plugin removed from $(PLUGIN_DIR)"
