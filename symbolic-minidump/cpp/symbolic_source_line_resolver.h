#ifndef cpp / symbolic_source_line_resolver_h_INCLUDED
#define cpp / symbolic_source_line_resolver_h_INCLUDED

#include "google_breakpad/processor/source_line_resolver_base.h"

namespace google_breakpad {
class SymbolicSourceLineResolver : public SourceLineResolverBase {
   public:
    SymbolicSourceLineResolver(bool is_big_endian);
    virtual ~SymbolicSourceLineResolver() {
    }

    using SourceLineResolverBase::FillSourceLineInfo;
    using SourceLineResolverBase::FindCFIFrameInfo;
    using SourceLineResolverBase::FindWindowsFrameInfo;
    using SourceLineResolverBase::HasModule;
    using SourceLineResolverBase::IsModuleCorrupt;
    using SourceLineResolverBase::LoadModule;
    using SourceLineResolverBase::LoadModuleUsingMapBuffer;
    using SourceLineResolverBase::LoadModuleUsingMemoryBuffer;
    using SourceLineResolverBase::ShouldDeleteMemoryBufferAfterLoadModule;
    using SourceLineResolverBase::UnloadModule;

    bool is_big_endian() {
        return is_big_endian_;
    }

   private:
    bool is_big_endian_;
    // friend declarations:
    friend class BasicModuleFactory;
    friend class ModuleComparer;
    friend class ModuleSerializer;
    template <class>
    friend class SimpleSerializer;

    // Function derives from SourceLineResolverBase::Function.
    struct Function;
    // Module implements SourceLineResolverBase::Module interface.
    class Module;

    // Disallow unwanted copy ctor and assignment operator
    SymbolicSourceLineResolver(const SymbolicSourceLineResolver &);
    void operator=(const SymbolicSourceLineResolver &);
};

#endif  // cpp/symbolic_source_line_resolver_h_INCLUDED
}
