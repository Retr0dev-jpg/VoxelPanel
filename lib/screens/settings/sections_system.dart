// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'dart:convert';
import 'dart:io';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:voxel_panel/screens/settings/launcher_settings_screen.dart';
import 'package:voxel_panel/screens/settings/settings_widgets.dart';
import 'package:voxel_panel/src/desktop_integration.dart';
import 'package:voxel_panel/src/l10n.dart';
import 'package:voxel_panel/src/rust/api/files.dart';
import 'package:voxel_panel/src/rust/api/panel.dart';
import 'package:voxel_panel/src/rust/api/types.dart';
import 'package:voxel_panel/src/settings.dart';
import 'package:voxel_panel/src/theme.dart';
import 'package:voxel_panel/widgets/common/feedback.dart';

const repositoryUrl = 'https://github.com/Retr0dev-jpg/VoxelPanel';

/// GitHub "new issue" page with the environment prefilled in the app language.
Uri newIssueUri(AppLocalizations l, String version) {
  final system = '${Platform.operatingSystem} ${Platform.operatingSystemVersion}';
  return Uri.parse('$repositoryUrl/issues/new').replace(queryParameters: {'body': l.issueTemplate(version, system)});
}

final javaRuntimesProvider = FutureProvider.autoDispose<List<JavaRuntimeInfo>>((ref) => listRuntimes());

class PathsSection extends ConsumerStatefulWidget {
  const PathsSection({super.key, required this.settings});

  final LauncherSettings settings;

  @override
  ConsumerState<PathsSection> createState() => _PathsSectionState();
}

class _PathsSectionState extends ConsumerState<PathsSection> {
  var _busy = false;

  Future<void> _move(ManagedFolder folder, String? destination) async {
    final l = context.l10n;
    final target = destination ?? await FilePicker.getDirectoryPath(dialogTitle: l.chooseFolder);
    if (target == null || !mounted) {
      return;
    }
    final ok = await confirmAction(context, title: l.moveFolderTitle, message: l.moveFolderMessage(target.isEmpty ? l.defaultFolder : target), confirmLabel: l.moveFolderConfirm);
    if (!ok || !mounted) {
      return;
    }
    setState(() => _busy = true);
    try {
      final saved = await moveManagedFolder(folder: folder, destination: target);
      ref.read(launcherSettingsProvider.notifier).replace(saved);
      if (mounted) {
        showMessage(context, l.moveFolderDone);
      }
    } catch (error) {
      if (mounted) {
        showError(context, error);
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final paths = appPaths();
    final custom = widget.settings.paths;
    Widget row(String title, String subtitle, ManagedFolder folder, String path, String stored) => SettingsGroup(
      title: title,
      subtitle: subtitle,
      children: [
        SelectableText(path, style: TextStyle(color: context.voxel.muted)),
        const SizedBox(height: 12),
        Wrap(
          spacing: 8,
          runSpacing: 8,
          children: [
            OutlinedButton.icon(onPressed: () => runGuarded(context, () => openInExplorer(path: path)), icon: const Icon(Icons.folder_open), label: Text(l.openFolder)),
            OutlinedButton.icon(onPressed: _busy ? null : () => _move(folder, null), icon: const Icon(Icons.drive_file_move_outline), label: Text(l.changeFolder)),
            if (stored.isNotEmpty) TextButton(onPressed: _busy ? null : () => _move(folder, ''), child: Text(l.restoreDefaultFolder)),
          ],
        ),
      ],
    );
    return Column(
      children: [
        if (_busy) const Padding(padding: EdgeInsets.only(bottom: 12), child: LinearProgressIndicator()),
        SettingsGroup(
          title: l.dataFolder,
          subtitle: l.dataFolderHint,
          children: [
            SelectableText(paths.data, style: TextStyle(color: context.voxel.muted)),
            const SizedBox(height: 12),
            Align(
              alignment: Alignment.centerLeft,
              child: OutlinedButton.icon(onPressed: () => runGuarded(context, () => openInExplorer(path: paths.data)), icon: const Icon(Icons.folder_open), label: Text(l.openFolder)),
            ),
          ],
        ),
        row(l.serversFolder, l.serversFolderHint, ManagedFolder.servers, paths.servers, custom.serversDir),
        row(l.backupsFolder, l.backupsFolderHint, ManagedFolder.backups, paths.backups, custom.backupsDir),
        row(l.runtimesFolder, l.runtimesFolderHint, ManagedFolder.runtimes, paths.runtimes, custom.runtimesDir),
        row(l.cacheFolder, l.cacheFolderHint, ManagedFolder.cache, paths.cache, custom.cacheDir),
      ],
    );
  }
}

class JavaSection extends ConsumerStatefulWidget {
  const JavaSection({super.key, required this.settings});

  final LauncherSettings settings;

  @override
  ConsumerState<JavaSection> createState() => _JavaSectionState();
}

class _JavaSectionState extends ConsumerState<JavaSection> {
  String? _progress;

  Future<void> _install() async {
    final choice = await showDialog<(JavaVendor, int)>(
      context: context,
      builder: (context) => _JavaInstallDialog(initialVendor: widget.settings.java.vendor),
    );
    if (choice == null || !mounted) {
      return;
    }
    final (vendor, major) = choice;
    try {
      await for (final event in installJava(major: major, vendor: vendor)) {
        if (!mounted) {
          return;
        }
        setState(() => _progress = event.message);
        if (event.error != null) {
          throw Exception(event.error);
        }
      }
    } catch (error) {
      if (mounted) {
        showError(context, error);
      }
    } finally {
      if (mounted) {
        setState(() => _progress = null);
        ref.invalidate(javaRuntimesProvider);
      }
    }
  }

  void _setPreferred(int major, String? path) {
    final preferred = [
      for (final entry in widget.settings.java.preferred)
        if (entry.major != major) entry,
      if (path != null) PreferredJava(major: major, path: path),
    ]..sort((a, b) => b.major.compareTo(a.major));
    saveSettings(context, ref, widget.settings.copyWith(java: JavaSettings(preferred: preferred, vendor: widget.settings.java.vendor)));
  }

  void _setVendor(JavaVendor vendor) {
    saveSettings(context, ref, widget.settings.copyWith(java: JavaSettings(preferred: widget.settings.java.preferred, vendor: vendor)));
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final colors = context.voxel;
    final runtimes = ref.watch(javaRuntimesProvider);
    return runtimes.when(
      loading: () => const Center(child: CircularProgressIndicator()),
      error: (error, stack) => ErrorState(error: error, onRetry: () => ref.invalidate(javaRuntimesProvider)),
      data: (runtimes) {
        final majors = runtimes.map((runtime) => runtime.major).where((major) => major > 0).toSet().toList()..sort((a, b) => b.compareTo(a));
        final vendors = {for (final info in javaVendors()) info.vendor: info};
        return Column(
          children: [
            SettingsGroup(
              title: l.javaVendor,
              subtitle: l.javaVendorHint,
              children: [
                SettingsRow(
                  title: l.javaVendorLabel,
                  child: SettingsDropdown<JavaVendor>(
                    value: widget.settings.java.vendor,
                    items: {for (final info in vendors.values) info.vendor: '${info.name} · ${info.publisher}'},
                    onChanged: _setVendor,
                  ),
                ),
                if (vendors[widget.settings.java.vendor] case final info?)
                  Align(
                    alignment: Alignment.centerLeft,
                    child: TextButton.icon(onPressed: () => launchUrl(Uri.parse(info.website)), icon: const Icon(Icons.open_in_new, size: 16), label: Text(l.javaVendorWebsite(info.name))),
                  ),
              ],
            ),
            SettingsGroup(
              title: l.javaInstalled,
              subtitle: l.javaInstalledHint,
              children: [
                Wrap(
                  spacing: 8,
                  children: [
                    FilledButton.icon(onPressed: _progress == null ? _install : null, icon: const Icon(Icons.download), label: Text(l.javaInstall)),
                    OutlinedButton.icon(onPressed: () => ref.invalidate(javaRuntimesProvider), icon: const Icon(Icons.refresh), label: Text(l.javaDetect)),
                  ],
                ),
                if (_progress != null) ...[const SizedBox(height: 12), const LinearProgressIndicator(), Text(_progress!, style: TextStyle(color: colors.muted))],
                const SizedBox(height: 8),
                if (runtimes.isEmpty) Padding(padding: const EdgeInsets.all(12), child: Text(l.javaNone, style: TextStyle(color: colors.muted))),
                for (final runtime in runtimes)
                  ListTile(
                    contentPadding: EdgeInsets.zero,
                    leading: Icon(Icons.coffee, color: runtime.managed ? colors.accent : colors.muted),
                    title: Text(runtime.major == 0 ? runtime.name : 'Java ${runtime.major} · ${runtime.name}'),
                    subtitle: Text(
                      '${runtime.managed ? l.javaManaged : runtime.system ? l.javaSystem : l.javaServer}'
                      '${runtime.vendor == null ? '' : ' · ${vendors[runtime.vendor]?.name ?? ''}'}\n${runtime.path}',
                    ),
                    isThreeLine: true,
                    trailing: runtime.managed
                        ? IconButton(
                            tooltip: l.delete,
                            icon: const Icon(Icons.delete_outline),
                            onPressed: () async {
                              final ok = await confirmAction(context, title: l.javaDeleteTitle, message: runtime.path, destructive: true, confirmLabel: l.delete);
                              if (ok && context.mounted && await runGuarded(context, () => deleteJavaRuntime(path: runtime.path))) {
                                ref.invalidate(javaRuntimesProvider);
                              }
                            },
                          )
                        : null,
                  ),
              ],
            ),
            SettingsGroup(
              title: l.javaPreferred,
              subtitle: l.javaPreferredHint,
              children: [
                for (final major in majors)
                  SettingsRow(
                    title: 'Java $major',
                    child: SettingsDropdown<String>(
                      value: widget.settings.java.preferred.where((entry) => entry.major == major).map((entry) => entry.path).firstOrNull ?? '',
                      items: {
                        '': l.javaAutomatic,
                        for (final runtime in runtimes.where((runtime) => runtime.major == major)) runtime.path: runtime.name,
                      },
                      onChanged: (value) => _setPreferred(major, value.isEmpty ? null : value),
                    ),
                  ),
              ],
            ),
          ],
        );
      },
    );
  }
}

/// Picks a distribution and a Java version; pops `(vendor, major)`.
class _JavaInstallDialog extends StatefulWidget {
  const _JavaInstallDialog({required this.initialVendor});

  final JavaVendor initialVendor;

  @override
  State<_JavaInstallDialog> createState() => _JavaInstallDialogState();
}

class _JavaInstallDialogState extends State<_JavaInstallDialog> {
  late JavaVendor _vendor = widget.initialVendor;
  late Future<List<JavaReleaseInfo>> _releases = listJavaReleases(vendor: _vendor);

  void _select(JavaVendor vendor) => setState(() {
    _vendor = vendor;
    _releases = listJavaReleases(vendor: vendor);
  });

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final colors = context.voxel;
    return AlertDialog(
      title: Text(l.javaInstall),
      content: SizedBox(
        width: 420,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            SettingsDropdown<JavaVendor>(value: _vendor, items: {for (final info in javaVendors()) info.vendor: '${info.name} · ${info.publisher}'}, onChanged: _select),
            const SizedBox(height: 12),
            Text(l.javaVersion, style: TextStyle(color: colors.muted, fontSize: 12)),
            const SizedBox(height: 4),
            SizedBox(
              height: 280,
              child: FutureBuilder<List<JavaReleaseInfo>>(
                future: _releases,
                builder: (context, snapshot) {
                  if (snapshot.hasError) {
                    return ErrorState(error: snapshot.error!, onRetry: () => _select(_vendor));
                  }
                  final releases = snapshot.data;
                  if (releases == null) {
                    return const Center(child: CircularProgressIndicator());
                  }
                  if (releases.isEmpty) {
                    return Center(child: Text(l.javaNoReleases, style: TextStyle(color: colors.muted)));
                  }
                  return ListView(
                    children: [
                      for (final release in releases)
                        ListTile(
                          dense: true,
                          leading: Icon(Icons.coffee, color: release.lts ? colors.accent : colors.muted),
                          title: Text('Java ${release.major}${release.lts ? ' LTS' : ''}'),
                          onTap: () => Navigator.pop(context, (_vendor, release.major)),
                        ),
                    ],
                  );
                },
              ),
            ),
          ],
        ),
      ),
      actions: [TextButton(onPressed: () => Navigator.pop(context), child: Text(l.cancel))],
    );
  }
}

class AdvancedSection extends ConsumerWidget {
  const AdvancedSection({super.key, required this.settings});

  final LauncherSettings settings;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = context.l10n;
    final advanced = settings.advanced;
    void save(AdvancedSettings next) => saveSettings(context, ref, settings.copyWith(advanced: next));
    return Column(
      children: [
        SettingsGroup(
          title: l.appLogs,
          subtitle: l.appLogsHint,
          children: [
            SettingsSwitch(title: l.appLogsEnabled, value: advanced.logging, onChanged: (value) => save(advanced.copyWith(logging: value))),
            SettingsRow(
              title: l.appLogLevel,
              child: SettingsDropdown<AppLogLevel>(
                value: advanced.logLevel,
                items: const {AppLogLevel.error: 'Error', AppLogLevel.warn: 'Warn', AppLogLevel.info: 'Info', AppLogLevel.debug: 'Debug'},
                onChanged: (value) => save(advanced.copyWith(logLevel: value)),
              ),
            ),
            Align(
              alignment: Alignment.centerLeft,
              child: OutlinedButton.icon(onPressed: () => runGuarded(context, () => openInExplorer(path: appLogDir())), icon: const Icon(Icons.folder_open), label: Text(l.openLogs)),
            ),
          ],
        ),
        SettingsGroup(
          title: l.maintenance,
          children: [
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                OutlinedButton.icon(
                  onPressed: () => runGuarded(context, clearCache, success: l.cacheCleared),
                  icon: const Icon(Icons.cleaning_services_outlined),
                  label: Text(l.clearCache),
                ),
                OutlinedButton.icon(
                  onPressed: () => runGuarded(context, () async {
                    final json = await exportLauncherSettings();
                    final saved = await FilePicker.saveFile(
                      dialogTitle: l.exportSettings,
                      fileName: 'voxelpanel-settings.json',
                      bytes: utf8.encode(json),
                      mimeType: 'application/json',
                      type: FileType.custom,
                      allowedExtensions: const ['json'],
                    );
                    if (saved != null && context.mounted) {
                      showMessage(context, l.settingsExported);
                    }
                  }),
                  icon: const Icon(Icons.upload_outlined),
                  label: Text(l.exportSettings),
                ),
                OutlinedButton.icon(
                  onPressed: () async {
                    final files = await FilePicker.pickFiles(dialogTitle: l.importSettings, type: FileType.custom, allowedExtensions: const ['json']);
                    final path = files.isEmpty ? null : files.first.path;
                    if (path == null || !context.mounted) {
                      return;
                    }
                    await runGuarded(context, () async {
                      final imported = await importLauncherSettings(path: path);
                      ref.read(launcherSettingsProvider.notifier).replace(imported);
                    }, success: l.settingsImported);
                  },
                  icon: const Icon(Icons.download_outlined),
                  label: Text(l.importSettings),
                ),
                OutlinedButton.icon(
                  style: OutlinedButton.styleFrom(foregroundColor: context.voxel.danger),
                  onPressed: () async {
                    final ok = await confirmAction(context, title: l.resetSettingsTitle, message: l.resetSettingsMessage, destructive: true, confirmLabel: l.reset);
                    if (ok && context.mounted) {
                      await runGuarded(context, () async {
                        ref.read(launcherSettingsProvider.notifier).replace(await resetLauncherSettings());
                      });
                    }
                  },
                  icon: const Icon(Icons.restart_alt),
                  label: Text(l.resetSettings),
                ),
              ],
            ),
          ],
        ),
      ],
    );
  }
}

class AboutSection extends ConsumerWidget {
  const AboutSection({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final l = context.l10n;
    final version = ref.watch(appVersionProvider).value ?? '…';
    return SettingsGroup(
      title: l.appTitle,
      subtitle: l.aboutDescription,
      children: [
        Text(l.aboutVersion(version), style: const TextStyle(fontWeight: FontWeight.w600)),
        const SizedBox(height: 4),
        Text(l.licenseLine, style: TextStyle(color: context.voxel.muted)),
        const SizedBox(height: 4),
        Align(
          alignment: Alignment.centerLeft,
          child: InkWell(
            onTap: () => launchUrl(Uri.parse(repositoryUrl)),
            child: Text(repositoryUrl, style: TextStyle(color: context.voxel.accent, decoration: TextDecoration.underline, decorationColor: context.voxel.accent)),
          ),
        ),
        const SizedBox(height: 16),
        Wrap(
          spacing: 8,
          runSpacing: 8,
          children: [
            OutlinedButton.icon(onPressed: () => launchUrl(Uri.parse(repositoryUrl)), icon: const Icon(Icons.code), label: Text(l.aboutRepository)),
            Tooltip(
              message: l.aboutReportIssueHint,
              child: OutlinedButton.icon(onPressed: () => launchUrl(newIssueUri(l, version)), icon: const Icon(Icons.bug_report_outlined), label: Text(l.aboutReportIssue)),
            ),
            OutlinedButton.icon(
              onPressed: () => showLicensePage(context: context, applicationName: 'VoxelPanel', applicationVersion: version, applicationLegalese: 'AGPL-3.0-or-later'),
              icon: const Icon(Icons.gavel),
              label: Text(l.aboutLicenses),
            ),
            OutlinedButton.icon(
              onPressed: () async {
                try {
                  final update = await checkForUpdate(currentVersion: version);
                  if (!context.mounted) {
                    return;
                  }
                  if (update == null) {
                    showMessage(context, l.upToDate);
                  } else {
                    showMessage(context, l.updateAvailable(update.version));
                    await launchUrl(Uri.parse(update.url));
                  }
                } catch (error) {
                  if (context.mounted) {
                    showError(context, error);
                  }
                }
              },
              icon: const Icon(Icons.system_update_alt),
              label: Text(l.checkUpdatesNow),
            ),
          ],
        ),
      ],
    );
  }
}
