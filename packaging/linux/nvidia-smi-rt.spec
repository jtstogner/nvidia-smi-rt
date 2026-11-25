%global debug_package %{nil}
%global _build_id_links none

Name:           nvidia-smi-rt
Version:        %{_version}
Release:        1%{?dist}
Summary:        Real-time NVIDIA GPU telemetry dashboard (TUI & GUI) and process manager

License:        MIT
URL:            https://github.com/jtstogner/nvidia-smi-rt

%description
nvidia-smi-rt interfaces directly with NVIDIA drivers via NVML (with automatic
nvidia-smi CLI fallback) to deliver low-overhead real-time GPU telemetry charts,
gauges, multi-GPU fleet views, and an interactive process manager in both TUI
(nvidia-smi-rt) and desktop GUI (nvidia-smi-rt-gui) variants.

%install
rm -rf %{buildroot}
install -Dm755 "%{_srcbindir}/nvidia-smi-rt" "%{buildroot}%{_bindir}/nvidia-smi-rt"
install -Dm755 "%{_srcbindir}/nvidia-smi-rt-gui" "%{buildroot}%{_bindir}/nvidia-smi-rt-gui"
install -Dm644 "%{_sourcedir}/nvidia-smi-rt-gui.desktop" "%{buildroot}%{_datadir}/applications/nvidia-smi-rt-gui.desktop"
install -Dm644 "%{_sourcedir}/nvidia-smi-rt.svg" "%{buildroot}%{_datadir}/icons/hicolor/scalable/apps/nvidia-smi-rt.svg"
install -Dm644 "%{_sourcedir}/LICENSE" "%{buildroot}%{_licensedir}/nvidia-smi-rt/LICENSE"
install -Dm644 "%{_sourcedir}/README.md" "%{buildroot}%{_docdir}/nvidia-smi-rt/README.md"

%files
%license %{_licensedir}/nvidia-smi-rt/LICENSE
%doc %{_docdir}/nvidia-smi-rt/README.md
%{_bindir}/nvidia-smi-rt
%{_bindir}/nvidia-smi-rt-gui
%{_datadir}/applications/nvidia-smi-rt-gui.desktop
%{_datadir}/icons/hicolor/scalable/apps/nvidia-smi-rt.svg
