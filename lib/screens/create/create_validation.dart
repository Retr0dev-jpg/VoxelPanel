// Copyright (C) 2026 Marco Simone Cannizzaro
// Licensed under the GNU Affero General Public License v3.0 or later.
// See the LICENSE file in the project root.

import 'package:voxel_panel/src/l10n.dart';

enum WizardStep { software, version, runtime, settings, summary }

enum CreateIssue { missingName, missingProvider, missingVersion, missingJar, invalidRam, invalidPort, invalidPlayers, eulaNotAccepted }

/// Everything the wizard collected, validated step by step before moving on.
class WizardInput {
  const WizardInput({
    required this.name,
    required this.hasProvider,
    required this.customJar,
    required this.isCustom,
    required this.version,
    required this.ramMinMb,
    required this.ramMaxMb,
    required this.port,
    required this.maxPlayers,
    required this.acceptEula,
    required this.needsEula,
  });

  final String name;
  final bool hasProvider;
  final bool isCustom;
  final String customJar;
  final String version;
  final int ramMinMb;
  final int ramMaxMb;

  /// 0 means "first free port".
  final int port;
  final int maxPlayers;
  final bool acceptEula;
  final bool needsEula;
}

CreateIssue? validateStep(WizardStep step, WizardInput input) {
  switch (step) {
    case WizardStep.software:
      if (input.name.trim().isEmpty) {
        return CreateIssue.missingName;
      }
      if (!input.hasProvider) {
        return CreateIssue.missingProvider;
      }
    case WizardStep.version:
      if (input.isCustom && input.customJar.isEmpty) {
        return CreateIssue.missingJar;
      }
      if (!input.isCustom && input.version.isEmpty) {
        return CreateIssue.missingVersion;
      }
    case WizardStep.runtime:
      if (input.ramMinMb < 256 || input.ramMaxMb < input.ramMinMb) {
        return CreateIssue.invalidRam;
      }
    case WizardStep.settings:
      if (input.port != 0 && (input.port < 1024 || input.port > 65535)) {
        return CreateIssue.invalidPort;
      }
      if (input.maxPlayers < 1 || input.maxPlayers > 100000) {
        return CreateIssue.invalidPlayers;
      }
    case WizardStep.summary:
      if (input.needsEula && !input.acceptEula) {
        return CreateIssue.eulaNotAccepted;
      }
  }
  return null;
}

/// First step that does not validate, or null when the whole wizard is complete.
CreateIssue? validateAll(WizardInput input) {
  for (final step in WizardStep.values) {
    final issue = validateStep(step, input);
    if (issue != null) {
      return issue;
    }
  }
  return null;
}

String createIssueText(AppLocalizations l, CreateIssue issue) => switch (issue) {
  CreateIssue.missingName => l.issueMissingName,
  CreateIssue.missingProvider => l.issueMissingProvider,
  CreateIssue.missingVersion => l.issueMissingVersion,
  CreateIssue.missingJar => l.issueMissingJar,
  CreateIssue.invalidRam => l.issueInvalidRam,
  CreateIssue.invalidPort => l.issueInvalidPort,
  CreateIssue.invalidPlayers => l.issueInvalidPlayers,
  CreateIssue.eulaNotAccepted => l.issueEula,
};

/// JVM memory value for a size in MiB (`4G` when it is a whole number of GiB).
String memoryValue(int megabytes) => megabytes % 1024 == 0 ? '${megabytes ~/ 1024}G' : '${megabytes}M';

int? parseMemoryMb(String value) {
  final match = RegExp(r'^(\d+)([MmGg])$').firstMatch(value.trim());
  if (match == null) {
    return null;
  }
  final number = int.parse(match.group(1)!);
  return match.group(2)!.toUpperCase() == 'G' ? number * 1024 : number;
}
