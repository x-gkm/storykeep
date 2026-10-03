// Client-side checks mirroring the API's validation rules; the server re-validates.

typedef Validator = String? Function(String? value);

Validator requiredText(String field, {required int max}) => (v) {
      final t = v?.trim() ?? '';
      if (t.isEmpty) return '$field is required';
      if (t.length > max) return '$field must be at most $max characters';
      return null;
    };

Validator optionalText(String field, {required int max}) => (v) {
      if ((v?.trim().length ?? 0) > max) return '$field must be at most $max characters';
      return null;
    };

String? validateEmail(String? v) {
  final t = v?.trim() ?? '';
  if (t.isEmpty) return 'Email is required';
  if (!RegExp(r'^[^@\s]+@[^@\s]+\.[^@\s]+$').hasMatch(t)) return 'Enter a valid email address';
  return null;
}

String? validatePassword(String? v) {
  final len = v?.length ?? 0;
  if (len < 8) return 'Password must be at least 8 characters';
  if (len > 128) return 'Password must be at most 128 characters';
  return null;
}

/// Parses a decimal typed with either `.` or `,`.
double? parseDecimal(String? v) => double.tryParse((v ?? '').trim().replaceAll(',', '.'));

/// Measurement value: > 0, < 10 000 000, at most 3 decimal places.
String? validateMeasurementValue(String? v) {
  final t = (v ?? '').trim().replaceAll(',', '.');
  if (t.isEmpty) return 'Value is required';
  final n = double.tryParse(t);
  if (n == null || !n.isFinite) return 'Enter a number';
  if (n <= 0) return 'Must be greater than 0';
  if (n >= 10000000) return 'Must be less than 10 000 000';
  final dot = t.indexOf('.');
  if (dot >= 0 && t.length - dot - 1 > 3) return 'At most 3 decimal places';
  return null;
}

String? validateTag(String name) {
  final t = name.trim();
  if (t.isEmpty) return 'Tag can\'t be blank';
  if (t.length > 50) return 'Tags are at most 50 characters';
  return null;
}
