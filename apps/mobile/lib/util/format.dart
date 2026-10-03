const _months = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

/// `FIRST_TIME` → `First time`.
String labelOf(String name) {
  final s = name.toLowerCase().split('_').join(' ');
  return s.isEmpty ? s : s[0].toUpperCase() + s.substring(1);
}

/// `Mar 14, 2025` for a calendar date.
String formatDate(DateTime d) => '${_months[d.month - 1]} ${d.day}, ${d.year}';

String monthYear(DateTime d) => '${_months[d.month - 1]} ${d.year}';

/// Local date and time of a UTC timestamp, e.g. `Mar 14, 2025 08:05`.
String formatDateTime(DateTime t) {
  final l = t.toLocal();
  return '${formatDate(l)} ${l.hour.toString().padLeft(2, '0')}:${l.minute.toString().padLeft(2, '0')}';
}

/// Compact countdown such as `2y 3mo`, `5d 4h`, `3h 12m` or `42s`.
String formatCountdown(Duration d) {
  if (d.inSeconds <= 0) return 'now';
  final days = d.inDays;
  if (days >= 365) {
    final years = days ~/ 365;
    final months = (days % 365) ~/ 30;
    return months > 0 ? '${years}y ${months}mo' : '${years}y';
  }
  if (days >= 30) return '${days ~/ 30}mo ${days % 30}d';
  if (days >= 1) return '${days}d ${d.inHours % 24}h';
  if (d.inHours >= 1) return '${d.inHours}h ${d.inMinutes % 60}m';
  if (d.inMinutes >= 1) return '${d.inMinutes}m ${d.inSeconds % 60}s';
  return '${d.inSeconds}s';
}

/// Age from a date of birth, e.g. `1 year 7 months` or `5 months`.
String formatAge(DateTime dob, [DateTime? now]) {
  final today = now ?? DateTime.now();
  var months = (today.year - dob.year) * 12 + today.month - dob.month;
  if (today.day < dob.day) months--;
  if (months < 0) return '';
  if (months < 1) return '${today.difference(dob).inDays} days';
  final years = months ~/ 12;
  final rest = months % 12;
  String plural(int n, String w) => '$n $w${n == 1 ? '' : 's'}';
  if (years == 0) return plural(rest, 'month');
  return rest == 0 ? plural(years, 'year') : '${plural(years, 'year')} ${plural(rest, 'month')}';
}

String formatBytes(int? bytes) {
  if (bytes == null) return '';
  if (bytes < 1024) return '$bytes B';
  if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
  return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
}

/// Trims trailing zeros: 74.250 → `74.25`, 72.0 → `72`.
String formatNumber(double v) {
  var s = v.toStringAsFixed(3).replaceFirst(RegExp(r'0+$'), '');
  if (s.endsWith('.')) s = s.substring(0, s.length - 1);
  return s;
}

/// Today's date at local midnight.
DateTime today() {
  final n = DateTime.now();
  return DateTime(n.year, n.month, n.day);
}
