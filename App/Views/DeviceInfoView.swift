import SwiftUI
import HomeiKit

struct DeviceInfoView: View {
    @EnvironmentObject var model: AppModel

    var body: some View {
        Screen(titleKey: "device.title") {
            if let p = model.fixture?.profile {
                ProvenanceBanner(provenance: p.provenance)
                SectionCard(titleKey: "device.fields") {
                    InfoRow(labelKey: "field.product_type", value: p.productType)
                    InfoRow(labelKey: "field.product_version", value: p.productVersion)
                    InfoRow(labelKey: "field.build_version", value: p.buildVersion)
                    InfoRow(labelKey: "field.device_class", value: p.deviceClass ?? String(localized: "state.not_provided"))
                    InfoRow(labelKey: "field.hardware_model", value: p.hardwareModel ?? String(localized: "state.not_provided"))
                    InfoRow(labelKey: "field.cpu_architecture", value: p.cpuArchitecture ?? String(localized: "state.not_provided"))
                }
                Text("device.fields_note").font(.footnote).foregroundStyle(Theme.textSecondary)
            }
        }
    }
}
