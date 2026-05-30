# Builds libpylon-cabi.so/.dylib locally (requires the pylon SDK).
#
# Usage:
#   make                         # build the shim
#   make PYLON_ROOT=/opt/pylon   # override pylon installation path (Linux)
#   make clean                   # remove build artefacts
#
# After building, tell cargo where to find the shim:
#   export PYLON_CABI=$(pwd)/libpylon-cabi.so   # Linux
#   export PYLON_CABI=$(pwd)/libpylon-cabi.dylib # macOS
#   cargo build

UNAME_S := $(shell uname -s)

CXX    ?= g++
CXXSTD ?= -std=c++14

SRC := src/libpylon-cabi.cc
HDR := include/libpylon-cabi.h

ifeq ($(UNAME_S),Darwin)
  PYLON_FW    ?= /Library/Frameworks/pylon.framework
  PYLON_INC   := $(PYLON_FW)/Headers
  PYLON_LIB   := $(PYLON_FW)/Libraries

  # -F adds the framework parent so `#include "pylon/Foo.h"` resolves via
  # pylon.framework/Headers/Foo.h through clang's framework search.
  # GenICam headers live under Headers/GenICam/ so add that too.
  CXXFLAGS := $(CXXSTD) -fPIC -O2 -Wall \
              -F$(PYLON_FW)/.. \
              -I$(PYLON_INC)/GenICam \
              -Iinclude

  # libpylonbase install name uses @rpath/pylon.framework/..., so rpath must
  # point to the parent directory of the framework, not the Libraries dir.
  LDFLAGS  := -dynamiclib \
              -L$(PYLON_LIB) \
              -Wl,-rpath,$(PYLON_FW)/.. \
              -lpylonbase \
              -lpylonutility \
              -lGCBase_gcc_v3_1_Basler_pylon \
              -lGenApi_gcc_v3_1_Basler_pylon

  OUT := libpylon-cabi.dylib
else
  PYLON_ROOT  ?= /opt/pylon
  PYLON_INC   := $(PYLON_ROOT)/include
  PYLON_LIB   := $(PYLON_ROOT)/lib

  CXXFLAGS := $(CXXSTD) -fPIC -O2 -Wall \
              -I$(PYLON_INC) \
              -Iinclude

  LDFLAGS  := -shared \
              -L$(PYLON_LIB) \
              -Wl,-rpath,$(PYLON_LIB) \
              -lpylonbase \
              -lpylonutility

  OUT := libpylon-cabi.so
endif

.PHONY: all clean

all: $(OUT)

$(OUT): $(SRC) $(HDR)
	$(CXX) $(CXXFLAGS) -o $@ $< $(LDFLAGS)
	@echo ""
	@echo "Built $(OUT).  Set the env var before cargo build:"
	@echo "  export PYLON_CABI=\$$(pwd)/$(OUT)"

clean:
	rm -f libpylon-cabi.dylib libpylon-cabi.so
