enum BaseFontPlatform {
  ios(17.0),
  android(16.0),
  macos(13.0),
  windows(15.0),
  linux(15.0);

  final double base;
  const BaseFontPlatform(this.base);
}

// Font sizes for iOS
enum FontSizes {
  large5(30.63),
  large4(27.23),
  large3(24.21),
  large2(21.52),
  large1(19.13),
  base(17.00),
  small1(15.11),
  small2(13.43);

  final double size;
  const FontSizes(this.size);
}

enum LabelFontSize {
  large2(FontSizes.large2),
  large1(FontSizes.large1),
  base(FontSizes.base),
  small1(FontSizes.small1),
  small2(FontSizes.small2);

  final FontSizes ref;
  const LabelFontSize(this.ref);
  double get size => ref.size;
}

enum BodyFontSize {
  large2(FontSizes.large2),
  large1(FontSizes.large1),
  base(FontSizes.base),
  small1(FontSizes.small1),
  small2(FontSizes.small2);

  final FontSizes ref;
  const BodyFontSize(this.ref);
  double get size => ref.size;
}

enum HeaderFontSize {
  h1(FontSizes.large5),
  h2(FontSizes.large4),
  h3(FontSizes.large3),
  h4(FontSizes.large2),
  h5(FontSizes.large1),
  h6(FontSizes.base);

  final FontSizes ref;
  const HeaderFontSize(this.ref);
  double get size => ref.size;
}
