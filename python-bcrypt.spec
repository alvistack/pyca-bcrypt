# Copyright 2025 Wong Hoi Sing Edison <hswong3i@pantarei-design.com>
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

%global debug_package %{nil}

%global source_date_epoch_from_changelog 0

Name: python-bcrypt
Epoch: 100
Version: 4.0.1
Release: 1%{?dist}
Summary: Modern(-ish) password hashing for your software and your servers
License: Apache-2.0
URL: https://github.com/pyca/bcrypt/tags
Source0: %{name}_%{version}.orig.tar.gz
BuildRequires: cargo
BuildRequires: fdupes
BuildRequires: gcc
BuildRequires: python3-Cython3
BuildRequires: python3-devel
BuildRequires: python3-pip
BuildRequires: python3-pycparser
BuildRequires: python3-setuptools
BuildRequires: python3-setuptools-rust
BuildRequires: python-rpm-macros
BuildRequires: rust >= 1.56.0

%description
Good password hashing for your software and your servers.

%prep
%autosetup -T -c -n %{name}_%{version}-%{release}
tar -zx -f %{S:0} --strip-components=1 -C .

%build
pip wheel \
    --no-deps \
    --no-build-isolation \
    --wheel-dir=dist \
    .

%install
pip install \
    --no-deps \
    --ignore-installed \
    --root=%{buildroot} \
    --prefix=%{_prefix} \
    dist/*.whl
find %{buildroot}%{python3_sitearch} -type f -name '*.pyc' -exec rm -rf {} \;
fdupes -qnrps %{buildroot}%{python3_sitearch}

%check

%if 0%{?suse_version} > 1500
%package -n python%{python3_version_nodots}-bcrypt
Summary: Modern(-ish) password hashing for your software and your servers
Requires: python3
Requires: python3-six >= 1.4.1
Provides: python3-bcrypt = %{epoch}:%{version}-%{release}
Provides: python3dist(bcrypt) = %{epoch}:%{version}-%{release}
Provides: python%{python3_version}-bcrypt = %{epoch}:%{version}-%{release}
Provides: python%{python3_version}dist(bcrypt) = %{epoch}:%{version}-%{release}
Provides: python%{python3_version_nodots}-bcrypt = %{epoch}:%{version}-%{release}
Provides: python%{python3_version_nodots}dist(bcrypt) = %{epoch}:%{version}-%{release}

%description -n python%{python3_version_nodots}-bcrypt
Good password hashing for your software and your servers.

%files -n python%{python3_version_nodots}-bcrypt
%license LICENSE
%{python3_sitearch}/*
%endif

%if !(0%{?suse_version} > 1500)
%package -n python3-bcrypt
Summary: Modern(-ish) password hashing for your software and your servers
Requires: python3
Requires: python3-six >= 1.4.1
Provides: python3-bcrypt = %{epoch}:%{version}-%{release}
Provides: python3dist(bcrypt) = %{epoch}:%{version}-%{release}
Provides: python%{python3_version}-bcrypt = %{epoch}:%{version}-%{release}
Provides: python%{python3_version}dist(bcrypt) = %{epoch}:%{version}-%{release}
Provides: python%{python3_version_nodots}-bcrypt = %{epoch}:%{version}-%{release}
Provides: python%{python3_version_nodots}dist(bcrypt) = %{epoch}:%{version}-%{release}

%description -n python3-bcrypt
Good password hashing for your software and your servers.

%files -n python3-bcrypt
%license LICENSE
%{python3_sitearch}/*
%endif

%changelog
