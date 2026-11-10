import CodyncKit
import SwiftUI

/// A navigation action inside the same settings cards used by bot profiles.
struct GroupSettingsAction: View {
    let title: String
    let icon: String
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 12) {
                Image(systemName: icon).font(.system(size: 18)).frame(width: 24)
                Text(L10n.tr(title)).frame(maxWidth: .infinity, alignment: .leading)
                Image(systemName: "chevron.right").font(.system(size: 11)).foregroundStyle(Palette.tertiary)
            }
            .foregroundStyle(Palette.text)
            .frame(minHeight: 32).contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }
}
