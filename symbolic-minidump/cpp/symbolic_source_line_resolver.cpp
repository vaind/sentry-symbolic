#include "cpp/symbolic_source_line_resolver.h"

#include "google_breakpad/processor/basic_source_line_resolver.h"
#include "google_breakpad/processor/source_line_resolver_interface.h"
#include "processor/basic_source_line_resolver_types.h"
#include "processor/cfi_frame_info.h"
#include "processor/module_factory.h"
using std::map;

namespace google_breakpad {

SymbolicSourceLineResolver::SymbolicSourceLineResolver(bool is_big_endian)
    : SourceLineResolverBase(new BasicModuleFactory) {
    is_big_endian_ = is_big_endian_;
}

CFIFrameInfo *SymbolicSourceLineResolver::Module::FindCFIFrameInfo(
    const StackFrame *frame) const {
    MemAddr address = frame->instruction - frame->module->base_address();
    MemAddr initial_base, initial_size;
    string initial_rules;

    // Find the initial rule whose range covers this address. That
    // provides an initial set of register recovery rules. Then, walk
    // forward from the initial rule's starting address to frame's
    // instruction address, applying delta rules.
    if (!cfi_initial_rules_.RetrieveRange(address, &initial_rules,
                                          &initial_base, NULL /* delta */,
                                          &initial_size)) {
        return NULL;
    }

    // Create a frame info structure, and populate it with the rules from
    // the STACK CFI INIT record.
    scoped_ptr<CFIFrameInfo> rules(new CFIFrameInfo());
    if (!ParseCFIRuleSet(initial_rules, rules.get())) return NULL;

    // Find the first delta rule that falls within the initial rule's range.
    map<MemAddr, string>::const_iterator delta =
        cfi_delta_rules_.lower_bound(initial_base);

    // Apply delta rules up to and including the frame's address.
    while (delta != cfi_delta_rules_.end() && delta->first <= address) {
        ParseCFIRuleSet(delta->second, rules.get());
        delta++;
    }

    return rules.release();
}
}  // namespace google_breakpad
